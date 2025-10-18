use heck::ToSnakeCase;
use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{
    Attribute, Field, Fields, Ident, ImplItem, ImplItemFn, ItemImpl, ItemStruct, Meta, Path, Token,
    Type, parse::Parse, parse::ParseStream, parse_macro_input, punctuated::Punctuated,
};

#[proc_macro_attribute]
pub fn state(attr: TokenStream, item: TokenStream) -> TokenStream {
    // Optional argument: `samples` to also derive samples and export sample constructor.
    let attr_ts = proc_macro2::TokenStream::from(attr);
    let mut enable_samples = false;
    let attr_trimmed = attr_ts.to_string().replace(' ', "");
    if !attr_trimmed.is_empty() {
        if attr_trimmed == "samples" {
            enable_samples = true;
        } else {
            return syn::Error::new_spanned(
                attr_ts,
                "`#[lera::state]` only supports optional `samples` argument, e.g. #[lera::state(samples)]",
            )
            .to_compile_error()
            .into();
        }
    }

    let mut item_struct = parse_macro_input!(item as ItemStruct);

    let record_path = parse_path("uniffi::Record");
    if let Err(err) = ensure_derive(&mut item_struct.attrs, &record_path) {
        return err.to_compile_error().into();
    }

    if enable_samples {
        let samples_path = parse_path("samples_derive::Samples");
        if let Err(err) = ensure_derive(&mut item_struct.attrs, &samples_path) {
            return err.to_compile_error().into();
        }
    }

    let struct_ident = item_struct.ident.clone();
    let struct_vis = item_struct.vis.clone();

    let fn_name_new_default =
        format_ident!("new_default_{}", struct_ident.to_string().to_snake_case());
    let listener_ident = format_ident!("{}ChangeListener", struct_ident);
    let fn_name_new_samples =
        format_ident!("new_{}_samples", struct_ident.to_string().to_snake_case());

    let expanded = if enable_samples {
        quote! {
            #item_struct

            #[uniffi::export]
            #struct_vis fn #fn_name_new_default() -> #struct_ident {
                #struct_ident::default()
            }

            // Export a sample-constructor function only when Samples is enabled for this state.
            #[uniffi::export]
            #struct_vis fn #fn_name_new_samples(n: u8) -> Vec<#struct_ident> {
                use samples_core::Samples;
                #struct_ident::sample_vec_n(n)
            }

            #[uniffi::export(with_foreign)]
            #struct_vis trait #listener_ident: Send + Sync {
                fn on_state_change(&self, state: #struct_ident);
            }

            ::lera::impl_state_change_listener_bridge!(#listener_ident, #struct_ident);
        }
    } else {
        quote! {
            #item_struct

            #[uniffi::export]
            #struct_vis fn #fn_name_new_default() -> #struct_ident {
                #struct_ident::default()
            }

            #[uniffi::export(with_foreign)]
            #struct_vis trait #listener_ident: Send + Sync {
                fn on_state_change(&self, state: #struct_ident);
            }

            ::lera::impl_state_change_listener_bridge!(#listener_ident, #struct_ident);
        }
    };

    expanded.into()
}

#[proc_macro_attribute]
pub fn default_params(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}

/// The `[lera::model]` procmacro creates ViewModels usable in Swift/Kotlin
#[proc_macro_attribute]
pub fn model(attr: TokenStream, item: TokenStream) -> TokenStream {
    let args = parse_macro_input!(attr as ModelArgs);
    let state_ty = args.state_ty;
    let has_navigator = args.has_navigator;

    let mut item_struct = parse_macro_input!(item as ItemStruct);
    let object_path = parse_path("uniffi::Object");
    if let Err(err) = ensure_derive(&mut item_struct.attrs, &object_path) {
        return err.to_compile_error().into();
    }

    let struct_ident = item_struct.ident.clone();

    let mut user_fields: Vec<Field> = Vec::new();
    match &mut item_struct.fields {
        Fields::Named(fields_named) => {
            for field in fields_named.named.iter() {
                user_fields.push(field.clone());
            }

            let state_field: Field = syn::parse_quote! {
                state: Arc<RwLock<#state_ty>>
            };
            let listener_ident = match type_last_segment_ident(&state_ty) {
                Ok(ident) => format_ident!("{}ChangeListener", ident),
                Err(err) => return err.to_compile_error().into(),
            };
            let listener_field: Field = syn::parse_quote! {
                state_change_listener: Arc<dyn #listener_ident>
            };

            fields_named.named.clear();
            fields_named.named.push(state_field);
            if has_navigator {
                let navigator_field: Field = syn::parse_quote! {
                    navigator: Arc<Navigator>
                };
                fields_named.named.push(navigator_field);
            }
            fields_named.named.push(listener_field);
            for field in user_fields.iter() {
                fields_named.named.push(field.clone());
            }
        }
        Fields::Unnamed(_) | Fields::Unit => {
            return syn::Error::new_spanned(
                &item_struct,
                "`#[lera::model]` expects a struct with named fields",
            )
            .to_compile_error()
            .into();
        }
    }

    let listener_ident = match type_last_segment_ident(&state_ty) {
        Ok(ident) => format_ident!("{}ChangeListener", ident),
        Err(err) => return err.to_compile_error().into(),
    };

    let user_field_inits: Vec<proc_macro2::TokenStream> = user_fields
        .iter()
        .map(|field| {
            let ident = field.ident.as_ref().expect("named field must have ident");
            quote! { #ident: Default::default() }
        })
        .collect();

    let has_background_task = user_fields.iter().any(|field| {
        field
            .ident
            .as_ref()
            .map(|name| name == "background_task")
            .unwrap_or(false)
    });

    let has_non_eq_field = user_fields.iter().any(|field| {
        field
            .ident
            .as_ref()
            .map(|name| name == "non_eq")
            .unwrap_or(false)
    });

    let has_non_hash_field = user_fields.iter().any(|field| {
        field
            .ident
            .as_ref()
            .map(|name| name == "non_hash")
            .unwrap_or(false)
    });

    let state_ty_clone = state_ty.clone();

    let navigator_deps_ty = if has_navigator {
        quote! { Arc<dyn ListenerOfNavigationChangesMadeByRust> }
    } else {
        quote! { () }
    };

    let navigator_field_init = if has_navigator {
        quote! { navigator: Arc::new(Navigator::new(navigator_listener_on_ffi_side.clone())), }
    } else {
        quote! {}
    };

    let make_self = quote! {
        Arc::new(Self {
            state: Arc::new(RwLock::new(state)),
            #navigator_field_init
            state_change_listener: listener,
            #(#user_field_inits,)*
        })
    };

    let new_body = if has_background_task {
        quote! {
            let should_start_auto_increment = state.is_auto_incrementing;
            let counter = #make_self;
            if should_start_auto_increment {
                counter.start_auto_incrementing();
            }
            counter
        }
    } else {
        make_self
    };

    let mut default_generics = item_struct.generics.clone();
    if !user_fields.is_empty() {
        let where_clause = default_generics.make_where_clause();
        for field in &user_fields {
            let ty = &field.ty;
            where_clause
                .predicates
                .push(syn::parse_quote! { #ty: Default });
        }
    }
    let (default_impl_generics, default_ty_generics, default_where_clause) =
        default_generics.split_for_impl();

    let without_listener_generics = item_struct.generics.clone();
    let (
        without_listener_impl_generics,
        without_listener_ty_generics,
        without_listener_where_clause,
    ) = without_listener_generics.split_for_impl();

    let user_without_listener_params: Vec<proc_macro2::TokenStream> = user_fields
        .iter()
        .map(|field| {
            let ident = field.ident.as_ref().expect("named field must have ident");
            let ty = &field.ty;
            quote! { #ident: #ty }
        })
        .collect();

    let mut without_listener_params: Vec<proc_macro2::TokenStream> = Vec::new();
    if has_navigator {
        without_listener_params.push(quote! {
            navigator_listener_on_ffi_side: Self::NavigatorDeps
        });
    }
    without_listener_params.extend(user_without_listener_params.iter().cloned());

    let without_listener_field_inits: Vec<proc_macro2::TokenStream> = user_fields
        .iter()
        .map(|field| {
            let ident = field.ident.as_ref().expect("named field must have ident");
            quote! { #ident }
        })
        .collect();

    let navigator_without_listener_init = if has_navigator {
        quote! { navigator: Arc::new(Navigator::new(navigator_listener_on_ffi_side.clone())), }
    } else {
        quote! {}
    };

    let without_listener_impl = quote! {
        impl #without_listener_impl_generics #struct_ident #without_listener_ty_generics #without_listener_where_clause {
            pub fn without_listener(state: #state_ty #(, #without_listener_params)*) -> Self {
                let state_change_listener = Arc::new([<#struct_ident NoopListener>]::default());

                Self {
                    state: Arc::new(RwLock::new(state)),
                    #navigator_without_listener_init
                    state_change_listener,
                    #(#without_listener_field_inits,)*
                }
            }
        }
    };

    let user_field_default_values: Vec<proc_macro2::TokenStream> = user_fields
        .iter()
        .map(|_| quote! { Default::default() })
        .collect();

    let default_impl = if has_navigator {
        proc_macro2::TokenStream::new()
    } else {
        quote! {
            impl #default_impl_generics Default for #struct_ident #default_ty_generics #default_where_clause {
                fn default() -> Self {
                    Self::without_listener(
                        #state_ty::default()
                        #(, #user_field_default_values)*
                    )
                }
            }
        }
    };

    let eq_impl_tokens = if !has_non_eq_field {
        let eq_checks: Vec<proc_macro2::TokenStream> = user_fields
            .iter()
            .map(|field| {
                let ident = field
                    .ident
                    .as_ref()
                    .expect("named field must have ident")
                    .clone();
                quote! { ::core::cmp::PartialEq::eq(&self.#ident, &other.#ident) }
            })
            .collect();

        let state_compare = quote! {
            {
                if ::std::sync::Arc::ptr_eq(&self.state, &other.state) {
                    true
                } else {
                    let self_ptr = ::std::sync::Arc::as_ptr(&self.state) as usize;
                    let other_ptr = ::std::sync::Arc::as_ptr(&other.state) as usize;
                    if self_ptr < other_ptr {
                        let self_state = self.state.read().unwrap();
                        let other_state = other.state.read().unwrap();
                        *self_state == *other_state
                    } else if self_ptr > other_ptr {
                        let other_state = other.state.read().unwrap();
                        let self_state = self.state.read().unwrap();
                        *self_state == *other_state
                    } else {
                        let self_state = self.state.read().unwrap();
                        let other_state = other.state.read().unwrap();
                        *self_state == *other_state
                    }
                }
            }
        };

        let mut partial_eq_generics = item_struct.generics.clone();
        {
            let where_clause = partial_eq_generics.make_where_clause();
            where_clause
                .predicates
                .push(syn::parse_quote! { #state_ty: ::core::cmp::PartialEq });
            for field in &user_fields {
                let ty = &field.ty;
                where_clause
                    .predicates
                    .push(syn::parse_quote! { #ty: ::core::cmp::PartialEq });
            }
        }
        let (partial_eq_impl_generics, partial_eq_ty_generics, partial_eq_where_clause) =
            partial_eq_generics.split_for_impl();

        let mut eq_generics = item_struct.generics.clone();
        {
            let where_clause = eq_generics.make_where_clause();
            where_clause
                .predicates
                .push(syn::parse_quote! { #state_ty: ::core::cmp::Eq });
            for field in &user_fields {
                let ty = &field.ty;
                where_clause
                    .predicates
                    .push(syn::parse_quote! { #ty: ::core::cmp::Eq });
            }
        }
        let (eq_impl_generics, eq_ty_generics, eq_where_clause) = eq_generics.split_for_impl();

        quote! {
            impl #partial_eq_impl_generics ::core::cmp::PartialEq for #struct_ident #partial_eq_ty_generics #partial_eq_where_clause {
                fn eq(&self, other: &Self) -> bool {
                    let state_equal = #state_compare;
                    state_equal
                        #(&& #eq_checks)*
                }
            }

            impl #eq_impl_generics ::core::cmp::Eq for #struct_ident #eq_ty_generics #eq_where_clause {}
        }
    } else {
        proc_macro2::TokenStream::new()
    };

    let hash_impl_tokens = if !has_non_hash_field {
        let hash_statements: Vec<proc_macro2::TokenStream> = user_fields
            .iter()
            .map(|field| {
                let ident = field
                    .ident
                    .as_ref()
                    .expect("named field must have ident")
                    .clone();
                quote! { ::std::hash::Hash::hash(&self.#ident, state); }
            })
            .collect();

        let mut hash_generics = item_struct.generics.clone();
        {
            let where_clause = hash_generics.make_where_clause();
            where_clause
                .predicates
                .push(syn::parse_quote! { #state_ty: ::std::hash::Hash });
            for field in &user_fields {
                let ty = &field.ty;
                where_clause
                    .predicates
                    .push(syn::parse_quote! { #ty: ::std::hash::Hash });
            }
        }
        let (hash_impl_generics, hash_ty_generics, hash_where_clause) =
            hash_generics.split_for_impl();

        quote! {
            impl #hash_impl_generics ::std::hash::Hash for #struct_ident #hash_ty_generics #hash_where_clause {
                fn hash<H: ::std::hash::Hasher>(&self, state: &mut H) {
                    {
                        let state_guard = self.state.read().unwrap();
                        ::std::hash::Hash::hash(&*state_guard, state);
                    }
                    #(#hash_statements)*
                }
            }
        }
    } else {
        proc_macro2::TokenStream::new()
    };

    let mut debug_generics = item_struct.generics.clone();
    {
        let where_clause = debug_generics.make_where_clause();
        where_clause
            .predicates
            .push(syn::parse_quote! { #state_ty: ::core::fmt::Debug });
    }
    let (debug_impl_generics, debug_ty_generics, debug_where_clause) =
        debug_generics.split_for_impl();

    let debug_impl_tokens = quote! {
        impl #debug_impl_generics ::core::fmt::Debug for #struct_ident #debug_ty_generics #debug_where_clause {
            fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                let state = self
                    .state
                    .read()
                    .expect("LeraModel::Debug failed to acquire read lock");
                ::core::fmt::Debug::fmt(&*state, f)
            }
        }
    };

    let mut display_generics = item_struct.generics.clone();
    {
        let where_clause = display_generics.make_where_clause();
        where_clause
            .predicates
            .push(syn::parse_quote! { #state_ty: ::core::fmt::Debug });
    }
    let (display_impl_generics, display_ty_generics, display_where_clause) =
        display_generics.split_for_impl();

    let display_impl_tokens = quote! {
        impl #display_impl_generics ::core::fmt::Display for #struct_ident #display_ty_generics #display_where_clause {
            fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                let state = self
                    .state
                    .read()
                    .expect("LeraModel::Display failed to acquire read lock");
                ::lera::fmt_utils::fmt_model_state(&*state, f)
            }
        }
    };

    if !has_non_eq_field && !has_non_hash_field {
        let export_path = parse_path("uniffi::export");
        let has_export_attr = item_struct
            .attrs
            .iter()
            .any(|attr| attr.path() == &export_path);
        if !has_export_attr {
            item_struct.attrs.push(syn::parse_quote!(#[uniffi::export(
                Eq,
                Hash,
                Debug,
                Display
            )]));
        }
    }

    let navigator_param = if has_navigator {
        quote! { navigator_listener_on_ffi_side: Self::NavigatorDeps }
    } else {
        quote! { _navigator_listener_on_ffi_side: Self::NavigatorDeps }
    };

    let expanded = quote! {
        #item_struct

        #[cfg(test)]
        paste::paste! {
            #[derive(Default)]
            struct [<#struct_ident NoopListener>];

            impl #listener_ident for [<#struct_ident NoopListener>] {
                fn on_state_change(&self, _state: #state_ty) {
                    // No-op
                }
            }

            #without_listener_impl
            #default_impl
        }


        impl ::lera::LeraModel for #struct_ident {
            type State = #state_ty;
            type Listener = Arc<dyn #listener_ident>;
            type NavigatorDeps = #navigator_deps_ty;

            fn new(
                state: Self::State,
                listener: Self::Listener,
                #navigator_param,
            ) -> Arc<Self> {
                #new_body
            }

            fn get_state_change_listener(&self) -> &Self::Listener {
                &self.state_change_listener
            }

            fn get_state_guard(&self) -> &Arc<RwLock<#state_ty_clone>> {
                &self.state
            }
        }

        #eq_impl_tokens
        #hash_impl_tokens
        #debug_impl_tokens
        #display_impl_tokens
    };
    expanded.into()
}

#[proc_macro_attribute]
pub fn api(attr: TokenStream, item: TokenStream) -> TokenStream {
    let args = if attr.is_empty() {
        ApiArgs::default()
    } else {
        parse_macro_input!(attr as ApiArgs)
    };

    let mut item_impl = parse_macro_input!(item as ItemImpl);
    if item_impl.trait_.is_some() {
        return syn::Error::new_spanned(
            &item_impl,
            "`#[lera::api]` can only be used on inherent impl blocks",
        )
        .to_compile_error()
        .into();
    }

    // Add `#[uniffi::export]` to the impl block if not already present
    if !item_impl
        .attrs
        .iter()
        .any(|attr| attr.path().is_ident("uniffi::export"))
    {
        item_impl.attrs.push(syn::parse_quote!(#[uniffi::export]));
    }

    // // Add `#[async_trait::async_trait]` to the impl block if not already present
    // // so we can support async methods
    // if !item_impl
    //     .attrs
    //     .iter()
    //     .any(|attr| attr.path().is_ident("async_trait"))
    // {
    //     item_impl.attrs.push(syn::parse_quote!(#[async_trait::async_trait]));
    // }

    let self_ty = item_impl.self_ty.as_ref();
    let struct_ident = match self_ty {
        Type::Path(type_path) => type_path
            .path
            .segments
            .last()
            .map(|segment| segment.ident.clone()),
        _ => None,
    };

    let struct_ident = match struct_ident {
        Some(ident) => ident,
        None => {
            return syn::Error::new_spanned(self_ty, "Unsupported type for `#[lera::api]`")
                .to_compile_error()
                .into();
        }
    };

    let state_ident = format_ident!("{}State", struct_ident);
    let listener_ident = format_ident!("{}ChangeListener", state_ident);

    let has_constructor = item_impl.items.iter().any(|item| match item {
        ImplItem::Fn(method) => method.sig.ident == "with_state_and_listener",
        _ => false,
    });

    if !has_constructor {
        let constructor: ImplItemFn = if args.has_navigator {
            syn::parse_quote! {
                #[uniffi::constructor(name = "new")]
                pub fn with_state_and_listener(
                    state: #state_ident,
                    listener: Arc<dyn #listener_ident>,
                    navigator_listener_on_ffi_side: Arc<dyn ListenerOfNavigationChangesMadeByRust>,
                ) -> Arc<Self> {
                    Self::new(state, listener, navigator_listener_on_ffi_side)
                }
            }
        } else {
            syn::parse_quote! {
                #[uniffi::constructor(name = "new")]
                pub fn with_state_and_listener(
                    state: #state_ident,
                    listener: Arc<dyn #listener_ident>,
                ) -> Arc<Self> {
                    Self::new(state, listener, ())
                }
            }
        };
        item_impl.items.insert(0, ImplItem::Fn(constructor));
        let state_getter: ImplItem = syn::parse_quote! {
            pub fn get_state(&self) -> #state_ident {
                let state_guard = self.state.read().expect("Failed to acquire read lock for state");
                state_guard.clone()
            }
        };
        item_impl.items.push(state_getter);
        let listener_getter: ImplItem = syn::parse_quote! {
            pub fn get_state_change_listener(&self) -> Arc<dyn #listener_ident> {
                self.state_change_listener.clone()
            }
        };
        item_impl.items.push(listener_getter);
    }

    quote! { #item_impl }.into()
}

#[derive(Default)]
struct ApiArgs {
    has_navigator: bool,
}

impl Parse for ApiArgs {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let ident: Ident = input.parse()?;
        if ident != "navigating" {
            return Err(syn::Error::new(
                ident.span(),
                "expected `navigating` argument, e.g. #[lera::api(navigating)]",
            ));
        }
        Ok(Self { has_navigator: true })
    }
}

struct ModelArgs {
    state_ty: Type,
    has_navigator: bool,
}

impl Parse for ModelArgs {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let key: Ident = input.parse()?;
        if key != "state" {
            return Err(syn::Error::new(
                key.span(),
                "expected `state` argument, e.g. #[lera::model(state = MyState)]",
            ));
        }

        input.parse::<Token![=]>()?;
        let state_ty: Type = input.parse()?;

        if input.peek(Token![,]) {
            input.parse::<Token![,]>()?;
            if input.is_empty() {
                return Err(input.error("unexpected comma (,) without additional arguments"));
            }
        }

        let has_navigator = match input.parse::<Ident>() {
            Ok(key) => {
                if key != "navigating" {
                    Err(syn::Error::new(
                        key.span(),
                        "expected `navigating` argument, e.g. #[lera::model(state = MyState, navigating)]",
                    ))
                } else {
                    Ok(true)
                }
            }
            Err(_) => Ok(false),
        }?;

        Ok(Self {
            state_ty,
            has_navigator,
        })
    }
}

fn ensure_derive(attrs: &mut Vec<Attribute>, derive_to_add: &Path) -> syn::Result<()> {
    for attr in attrs.iter_mut() {
        if attr.path().is_ident("derive") {
            let mut derives: Punctuated<Path, Token![,]> = match &attr.meta {
                Meta::List(list) => {
                    list.parse_args_with(Punctuated::<Path, Token![,]>::parse_terminated)?
                }
                meta => {
                    return Err(syn::Error::new_spanned(
                        meta,
                        "expected #[derive(...)] attribute",
                    ));
                }
            };
            if !derives.iter().any(|existing| existing == derive_to_add) {
                derives.push(derive_to_add.clone());
                let new_attr: Attribute = syn::parse_quote!(#[derive(#derives)]);
                *attr = new_attr;
            }
            return Ok(());
        }
    }

    attrs.push(syn::parse_quote!(#[derive(#derive_to_add)]));
    Ok(())
}

fn type_last_segment_ident(ty: &Type) -> syn::Result<Ident> {
    if let Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            return Ok(segment.ident.clone());
        }
    }
    Err(syn::Error::new_spanned(
        ty,
        "Unsupported state type for `#[lera::model]`",
    ))
}

fn parse_path(path: &str) -> Path {
    syn::parse_str(path).expect("valid path")
}
