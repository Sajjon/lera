// samples-derive/src/lib.rs
use proc_macro::TokenStream;
use proc_macro2::TokenTree;
use quote::{format_ident, quote};
use syn::{parse_macro_input, Attribute, Data, DeriveInput, Expr, Fields, LitStr, Token, Type};

#[derive(Clone)]
enum CustomSample {
    Direct(Expr),
    // Function path that will be applied to the expr to construct the field value.
    // Intended for const fns returning Result<#ty, E> (e.g., Interval::const_try_from(…)).
    ConstFn { expr: Expr, method: syn::Path },
}

#[proc_macro_derive(Samples, attributes(samples))]
pub fn derive_samples(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let ident = input.ident.clone();

    let Data::Struct(data_struct) = &input.data else {
        return syn::Error::new_spanned(&input, "Samples can only be derived for structs")
            .to_compile_error()
            .into();
    };
    let Fields::Named(fields_named) = &data_struct.fields else {
        return syn::Error::new_spanned(&input, "Samples supports only named fields for now")
            .to_compile_error()
            .into();
    };

    struct FieldInfo {
        name: syn::Ident,
        ty: Type,
        custom_samples: Option<Vec<CustomSample>>,
    }
    let mut infos = Vec::<FieldInfo>::new();
    for f in &fields_named.named {
        let name = f.ident.clone().unwrap();
        let ty = f.ty.clone();
        let custom_samples = match parse_custom_samples(&f.attrs) {
            Ok(samples) => samples,
            Err(err) => return err.to_compile_error().into(),
        };
        infos.push(FieldInfo {
            name,
            ty,
            custom_samples,
        });
    }

    // Trait bounds
    let mut where_bounds = Vec::<proc_macro2::TokenStream>::new();
    for info in &infos {
        if info.custom_samples.is_none() {
            let ty = &info.ty;
            where_bounds.push(quote! { #ty: samples_core::Samples });
        }
    }

    if infos.is_empty() {
        let expanded = quote! {
            impl samples_core::Samples for #ident {
                fn samples() -> samples_core::SampleIter<Self> {
                    Box::new(::std::iter::once(Self {}))
                }
            }
        };
        return TokenStream::from(expanded);
    }

    let mut prelude = Vec::<proc_macro2::TokenStream>::new();
    let mut candidate_idents = Vec::new();
    let mut loop_vars = Vec::new();
    let mut field_assignments_move = Vec::new();
    let mut field_assignments_clone = Vec::new();
    let mut index_idents = Vec::new();
    let mut len_idents = Vec::new();

    // Special-case: single-field struct with constructor method overrides that produce `Self`.
    // Trigger only when all constructor methods clearly refer to the struct type
    // (bare method name -> inherent; or path starting with the struct ident).
    let single_field_self_ctor = if infos.len() == 1 {
        if let Some(custom) = &infos[0].custom_samples {
            let ctor_methods: Vec<&syn::Path> = custom
                .iter()
                .filter_map(|c| match c {
                    CustomSample::ConstFn { method, .. } => Some(method),
                    _ => None,
                })
                .collect();
            if ctor_methods.is_empty() {
                false
            } else {
                ctor_methods.into_iter().all(|path| {
                    if path.leading_colon.is_none() && path.segments.len() == 1 {
                        true
                    } else {
                        path.segments
                            .first()
                            .map(|seg| seg.ident == ident)
                            .unwrap_or(false)
                    }
                })
            }
        } else {
            false
        }
    } else {
        false
    };

    for (i, info) in infos.iter().enumerate() {
        let cands = format_ident!("f{}_cands", i);
        let var = format_ident!("f{}_val", i);
        let idx = format_ident!("f{}_idx", i);
        let len = format_ident!("f{}_len", i);
        let ty = &info.ty;
        let name = &info.name;

        if let Some(custom_samples) = info.custom_samples.as_ref() {
            // If single field struct and overrides use constructor methods, build `Vec<Self>`.
            if single_field_self_ctor {
                let custom_exprs: Vec<_> = custom_samples
                    .iter()
                    .enumerate()
                    .map(|(j, sample)| match sample {
                        CustomSample::Direct(expr) => {
                            let expr = expr.clone();
                            quote! { ( #expr ) }
                        }
                        CustomSample::ConstFn { expr, method } => {
                            let expr = expr.clone();
                            let method = method.clone();
                            // Qualify bare identifiers against the struct type
                            let method_call = if method.leading_colon.is_none() && method.segments.len() == 1 {
                                let seg_ident = method.segments.first().unwrap().ident.clone();
                                quote! { <#ident>::#seg_ident }
                            } else {
                                quote! { #method }
                            };
                            let message = LitStr::new(
                                &format!(
                                    "failed to validate #[samples] value for field `{}`",
                                    name
                                ),
                                proc_macro2::Span::call_site(),
                            );
                            let struct_name = ident.to_string().to_uppercase();
                            let field_name = name.to_string().to_uppercase();
                            let const_ident = format_ident!("__SAMPLES_CONST_{}_{}_{}", struct_name, field_name, j);
                            quote! {
                                {
                                    const #const_ident: () = {
                                        let __value = #method_call(#expr);
                                        if samples_core::__private::const_result_is_err::<#ident, _>(&__value) {
                                            panic!(#message);
                                        }
                                    };
                                    match #method_call(#expr) {
                                        ::core::result::Result::Ok(v) => v,
                                        ::core::result::Result::Err(_) => unreachable!("checked in const"),
                                    }
                                }
                            }
                        }
                    })
                    .collect();
                prelude.push(quote! {
                    let #cands: ::std::vec::Vec<#ident> = vec![#(#custom_exprs),*];
                });
            } else {
                let custom_exprs: Vec<_> = custom_samples
                .iter()
                .enumerate()
                .map(|(j, sample)| match sample {
                    CustomSample::Direct(expr) => {
                        let expr = expr.clone();
                        quote! { (#expr) }
                    }
                    CustomSample::ConstFn { expr, method } => {
                        let expr = expr.clone();
                        let method = method.clone();
                        // If the provided method path is a bare identifier (e.g., `const_try_from`),
                        // qualify it as an inherent associated function on the field type: `<#ty>::const_try_from`.
                        // Otherwise, use the path as-is (e.g., `Interval::const_try_from` or a fully qualified path).
                        let method_call = if method.leading_colon.is_none() && method.segments.len() == 1 {
                            let seg_ident = method.segments.first().unwrap().ident.clone();
                            quote! { <#ty>::#seg_ident }
                        } else {
                            quote! { #method }
                        };
                        let message = LitStr::new(
                            &format!(
                                "failed to validate #[samples] value for field `{}`",
                                name
                            ),
                            proc_macro2::Span::call_site(),
                        );
                        let struct_name = ident.to_string().to_uppercase();
                        let field_name = name.to_string().to_uppercase();
                        let const_ident = format_ident!("__SAMPLES_CONST_{}_{}_{}", struct_name, field_name, j);
                        quote! {
                            {
                                // Compile-time validation that the provided expr produces a valid #ty
                                const #const_ident: () = {
                                    let __value = #method_call(#expr);
                                    if samples_core::__private::const_result_is_err::<#ty, _>(&__value) {
                                        panic!(#message);
                                    }
                                };
                                // Construct the actual value at runtime (handles Result-returning fns)
                                match #method_call(#expr) {
                                    ::core::result::Result::Ok(v) => v,
                                    ::core::result::Result::Err(_) => unreachable!("checked in const"),
                                }
                            }
                        }
                    }
                })
                .collect();
                prelude.push(quote! {
                    let #cands: ::std::vec::Vec<#ty> = vec![#(#custom_exprs),*];
                });
            }
        } else {
            prelude.push(quote! {
                let #cands: ::std::vec::Vec<#ty> =
                    <#ty as samples_core::Samples>::samples().collect();
            });
        }
        candidate_idents.push(cands.clone());
        loop_vars.push(var.clone());
        index_idents.push(idx.clone());
        len_idents.push(len.clone());

        field_assignments_move.push(quote! {
            #name: #var
        });
        field_assignments_clone.push(quote! {
            #name: #cands[#idx].clone()
        });
    }

    let empty_check = if candidate_idents.is_empty() {
        quote! { false }
    } else {
        let empties: Vec<_> = candidate_idents
            .iter()
            .map(|c| quote! { #c.is_empty() })
            .collect();
        quote! { false #( || #empties )* }
    };

    let body = if infos.len() == 1 {
        let cands = &candidate_idents[0];
        if single_field_self_ctor {
            quote! {
                if #cands.is_empty() {
                    return Box::new(::std::iter::empty());
                }
                let iter = #cands.into_iter();
                Box::new(iter)
            }
        } else {
            let assignments = &field_assignments_move;
            let var = &loop_vars[0];
            quote! {
                if #cands.is_empty() {
                    return Box::new(::std::iter::empty());
                }
                let iter = #cands.into_iter().map(|#var| Self {
                    #(#assignments,)*
                });
                Box::new(iter)
            }
        }
    } else if infos.len() <= 8 {
        let assignments = &field_assignments_move;
        let vars = &loop_vars;
        quote! {
            if #empty_check {
                return Box::new(::std::iter::empty());
            }
            let iter = samples_core::itertools::iproduct!(#(#candidate_idents.into_iter()),*)
                .map(|(#(#vars),*)| Self {
                    #(#assignments,)*
                });
            Box::new(iter)
        }
    } else {
        let assignments = &field_assignments_clone;
        let mut advance_blocks = Vec::new();
        for ((_, idx), len) in candidate_idents
            .iter()
            .zip(index_idents.iter())
            .zip(len_idents.iter())
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
        {
            advance_blocks.push(quote! {
                if carry {
                    #idx += 1;
                    if #idx < #len {
                        carry = false;
                    } else {
                        #idx = 0;
                    }
                }
            });
        }

        quote! {
            if #empty_check {
                return Box::new(::std::iter::empty());
            }
            #(let #len_idents = #candidate_idents.len();)*
            #(let mut #index_idents = 0usize;)*
            let mut done = false;
            let iter = ::std::iter::from_fn(move || {
                if done {
                    return None;
                }
                let value = Self {
                    #(#assignments,)*
                };
                let mut carry = true;
                #(#advance_blocks)*
                if carry {
                    done = true;
                }
                Some(value)
            });
            Box::new(iter)
        }
    };

    let where_clause = if where_bounds.is_empty() {
        quote! {}
    } else {
        quote! {
            where
                #(#where_bounds,)*
        }
    };

    let marker_ident = format_ident!("__SAMPLES_DERIVE_MARKER_{}", ident);

    let expanded = quote! {
        impl samples_core::Samples for #ident
        #where_clause
        {
            fn samples() -> samples_core::SampleIter<Self> {
                #(#prelude)*
                #body
            }
        }
        // Private marker emitted by the derive macro so other macros can
        // detect that `Samples` was explicitly derived for this type.
        #[allow(non_upper_case_globals)]
        const #marker_ident: () = ();
    };
    TokenStream::from(expanded)
}

fn parse_custom_samples(attrs: &[Attribute]) -> syn::Result<Option<Vec<CustomSample>>> {
    let mut found: Option<Vec<CustomSample>> = None;
    for attr in attrs {
        if attr.path().is_ident("samples") {
            if found.is_some() {
                return Err(syn::Error::new_spanned(
                    attr,
                    "duplicate #[samples(...)] attribute",
                ));
            }
            let parsed = attr.parse_args_with(parse_samples_attr)?;
            if parsed.is_empty() {
                return Err(syn::Error::new_spanned(
                    attr,
                    "#[samples(...)] requires at least one expression",
                ));
            }
            found = Some(parsed);
        }
    }
    Ok(found)
}

fn parse_samples_attr(input: syn::parse::ParseStream<'_>) -> syn::Result<Vec<CustomSample>> {
    let mut entries = Vec::new();
    let mut first = true;

    while !input.is_empty() {
        if !first {
            let _comma: Token![,] = input.parse()?;
            if input.is_empty() {
                break;
            }
        }
        first = false;

        if input.peek(syn::token::Bracket) {
            let content;
            syn::bracketed!(content in input);
            let mut exprs = Vec::new();
            while !content.is_empty() {
                exprs.push(parse_sample_expr(&content)?);
                if content.peek(Token![,]) {
                    let _ = content.parse::<Token![,]>()?;
                } else {
                    break;
                }
            }
            if exprs.is_empty() {
                return Err(syn::Error::new(
                    content.span(),
                    "expected at least one expression inside []",
                ));
            }
            let method = if input.peek(Token![->]) {
                input.parse::<Token![->]>()?;
                Some(input.parse::<syn::Path>()?)
            } else {
                None
            };
            for expr in exprs {
                entries.push(match &method {
                    Some(method) => CustomSample::ConstFn {
                        expr,
                        method: method.clone(),
                    },
                    None => CustomSample::Direct(expr),
                });
            }
        } else {
            let expr = parse_sample_expr(input)?;
            let method = if input.peek(Token![->]) {
                input.parse::<Token![->]>()?;
                Some(input.parse::<syn::Path>()?)
            } else {
                None
            };
            entries.push(match method {
                Some(method) => CustomSample::ConstFn { expr, method },
                None => CustomSample::Direct(expr),
            });

            if input.peek(Token![,]) {
                let fork = input.fork();
                let _ = fork.parse::<Token![,]>()?;
                if !fork.is_empty() && !fork.peek(syn::token::Bracket) {
                    return Err(input.error("multiple #[samples] values must be wrapped in [...]"));
                }
            }
        }
    }

    Ok(entries)
}

fn parse_sample_expr(input: syn::parse::ParseStream<'_>) -> syn::Result<Expr> {
    let mut tokens = proc_macro2::TokenStream::new();
    while !input.is_empty() {
        if input.peek(Token![,]) || input.peek(Token![->]) {
            break;
        }
        let tt = input.parse::<TokenTree>()?;
        tokens.extend(std::iter::once(tt));
    }
    if tokens.is_empty() {
        return Err(input.error("expected expression"));
    }
    syn::parse2(tokens)
}
