use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

use syn::{
    Attribute, Expr, FnArg, Item, ItemImpl, ItemStruct, Pat, ReturnType, Type, TypePath,
    Visibility,
    parse::{Parse, ParseStream},
    parse_file,
};

/// Represents the default value declared via `#[lera::default_params]`.
#[derive(Debug, Clone)]
pub enum DefaultParamValue {
    /// A concrete Rust expression provided via `= <expr>`.
    ExplicitExpr(Expr),
    /// No explicit expression provided, caller should infer a language-specific default.
    Infer,
}

/// Parameter metadata extracted from a model method.
#[derive(Debug, Clone)]
pub struct ParsedParam {
    pub name: String,
    pub ty: Type,
    pub default: Option<DefaultParamValue>,
}

/// Return type metadata extracted from a model method.
#[derive(Debug, Clone)]
pub struct ParsedReturnType {
    pub ty: Option<Type>,
    pub uses_result: bool,
}

/// Method metadata for a model annotated with `#[lera::api]`.
#[derive(Debug, Clone)]
pub struct ParsedMethod {
    pub rust_name: String,
    pub camel_name: String,
    pub params: Vec<ParsedParam>,
    pub return_type: ParsedReturnType,
    pub is_async: bool,
}

/// Parsed representation of a `#[lera::model]` implementation.
#[derive(Debug, Clone)]
pub struct ParsedModel {
    pub model_name: String,
    pub state_name: String,
    pub listener_name: String,
    pub default_state_fn: String,
    pub methods: Vec<ParsedMethod>,
    pub source_path: PathBuf,
}

pub fn to_camel_case(snake_case: &str) -> String {
    let mut result = String::new();
    let mut capitalize_next = false;

    for ch in snake_case.chars() {
        if ch == '_' {
            capitalize_next = true;
        } else if capitalize_next {
            result.extend(ch.to_uppercase());
            capitalize_next = false;
        } else {
            result.push(ch);
        }
    }

    result
}

pub fn to_default_state_fn_name(state_name: &str) -> String {
    let mut result = "newDefault".to_string();
    result.push_str(state_name);
    result
}

fn attr_is_lera(attr: &Attribute, name: &str) -> bool {
    let mut segments = attr.path().segments.iter();
    if let Some(first) = segments.next() {
        if first.ident == "lera" {
            if let Some(second) = segments.next() {
                return second.ident == name && segments.next().is_none();
            }
        }
    }
    false
}

fn has_lera_attr(attrs: &[Attribute], name: &str) -> bool {
    attrs.iter().any(|attr| attr_is_lera(attr, name))
}

fn has_lera_api(attrs: &[Attribute]) -> bool {
    has_lera_attr(attrs, "api")
}

#[derive(Default)]
struct DefaultParamArgs {
    pairs: HashMap<String, Option<Expr>>,
}

impl Parse for DefaultParamArgs {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let mut pairs = HashMap::new();

        while !input.is_empty() {
            let ident: syn::Ident = input.parse()?;
            let value = if input.peek(syn::Token![=]) {
                input.parse::<syn::Token![=]>()?;
                Some(input.parse::<Expr>()?)
            } else {
                None
            };

            pairs.insert(ident.to_string(), value);

            if input.peek(syn::Token![,]) {
                input.parse::<syn::Token![,]>()?;
            } else {
                break;
            }
        }

        Ok(Self { pairs })
    }
}

fn is_uniffi_constructor(attrs: &[Attribute]) -> bool {
    attrs.iter().any(|attr| {
        let segments = &attr.path().segments;
        segments.len() == 2 && segments[0].ident == "uniffi" && segments[1].ident == "constructor"
    })
}

pub fn parse_lera_models(path_to_target_rust_crate: &Path) -> Result<Vec<ParsedModel>, String> {
    let manifest_dir = path_to_target_rust_crate;
    let search_dirs = [manifest_dir.join("src")];

    let mut models = Vec::new();
    let mut inspected_dirs = Vec::new();

    for dir in search_dirs {
        inspected_dirs.push(dir.clone());
        if !dir.exists() {
            continue;
        }
        let mut dir_models = parse_models_in_dir(&dir)?;
        models.append(&mut dir_models);
    }

    if models.is_empty() {
        let formatted_dirs = inspected_dirs
            .into_iter()
            .map(|dir| dir.display().to_string())
            .collect::<Vec<_>>()
            .join(", ");
        return Err(format!(
            "No #[lera::model] usages found in any of: {}",
            formatted_dirs
        ));
    }

    Ok(models)
}

fn parse_models_in_dir(dir: &Path) -> Result<Vec<ParsedModel>, String> {
    let entries = fs::read_dir(dir)
        .map_err(|e| format!("Failed to read models directory {}: {}", dir.display(), e))?;

    let mut models = Vec::new();

    for entry in entries {
        let entry = entry
            .map_err(|e| format!("Failed to read directory entry in {}: {}", dir.display(), e))?;
        let path = entry.path();

        if path.extension().and_then(|s| s.to_str()) == Some("rs") {
            let content = fs::read_to_string(&path)
                .map_err(|e| format!("Failed to read {:?}: {}", path, e))?;

            let syntax_tree =
                parse_file(&content).map_err(|e| format!("Failed to parse {:?}: {}", path, e))?;

            let file_models = parse_file_for_lera_models(&syntax_tree, &path)?;
            models.extend(file_models);
        }
    }

    Ok(models)
}

fn parse_file_for_lera_models(
    syntax_tree: &syn::File,
    file_path: &Path,
) -> Result<Vec<ParsedModel>, String> {
    let mut models = Vec::new();

    for item in &syntax_tree.items {
        if let Item::Struct(ItemStruct { ident, attrs, .. }) = item {
            if has_lera_attr(attrs, "model") {
                let state_name = attrs
                    .iter()
                    .find(|attr| attr_is_lera(attr, "model"))
                    .map(|attr| {
                        attr.parse_args::<ModelAttrArgs>().map_err(|e| {
                            format!(
                                "Failed to parse #[lera::model] attribute on {} in {:?}: {}",
                                ident, file_path, e
                            )
                        })
                    })
                    .transpose()? // Option<Result<...>> -> Result<Option<...>>
                    .map(|args| type_to_string(&args.state_ty))
                    .ok_or_else(|| {
                        format!(
                            "#[lera::model] attribute on {} in {:?} must specify a state",
                            ident, file_path
                        )
                    })?;

                let model_info = collect_model_info(ident, &state_name, syntax_tree, file_path)?;
                models.push(model_info);
            }
        }
    }

    Ok(models)
}

fn collect_model_info(
    model_ident: &syn::Ident,
    state_name: &str,
    syntax_tree: &syn::File,
    file_path: &Path,
) -> Result<ParsedModel, String> {
    let model_name = model_ident.to_string();
    let listener_name = format!("{}ChangeListener", state_name);
    let mut found_state_struct = false;
    let mut found_api_impl = false;
    let mut methods = Vec::new();

    for item in &syntax_tree.items {
        match item {
            Item::Struct(ItemStruct { ident, attrs, .. }) => {
                if ident == model_ident && !has_lera_attr(attrs, "model") {
                    return Err(format!(
                        "ACTIONABLE ERROR: struct {} must use #[lera::model] in {:?}",
                        model_name, file_path
                    ));
                }

                if *ident == state_name {
                    if !has_lera_attr(attrs, "state") {
                        return Err(format!(
                            "ACTIONABLE ERROR: state struct {} must use #[lera::state] in {:?}",
                            state_name, file_path
                        ));
                    }
                    found_state_struct = true;
                }
            }
            Item::Impl(ItemImpl {
                self_ty,
                attrs,
                items,
                ..
            }) => {
                if has_lera_api(attrs) {
                    if let Type::Path(TypePath { path, .. }) = self_ty.as_ref() {
                        if path
                            .segments
                            .last()
                            .map(|s| &s.ident)
                            .map(|ident| ident == model_ident)
                            .unwrap_or(false)
                        {
                            found_api_impl = true;
                            for impl_item in items {
                                if let syn::ImplItem::Fn(method) = impl_item {
                                    if let Visibility::Public(_) = method.vis {
                                        if is_uniffi_constructor(&method.attrs) {
                                            continue;
                                        }

                                        let method_name = method.sig.ident.to_string();
                                        let camel_name = to_camel_case(&method_name);

                                        let defaults_map = method
                                            .attrs
                                            .iter()
                                            .find(|attr| attr_is_lera(attr, "default_params"))
                                            .map(|attr| {
                                                attr.parse_args::<DefaultParamArgs>().map_err(|e| {
                                                    format!(
                                                        "Failed to parse #[lera::default_params] on {} in {:?}: {}",
                                                        method_name, file_path, e
                                                    )
                                                })
                                            })
                                            .transpose()? // Option<Result<...>> -> Result<Option<...>>
                                            .map(|args| args.pairs);

                                        let params = parse_method_parameters(
                                            &method.sig,
                                            defaults_map.as_ref(),
                                        );
                                        let return_metadata = parse_return_metadata(&method.sig);
                                        let is_async = method.sig.asyncness.is_some();

                                        methods.push(ParsedMethod {
                                            rust_name: method_name,
                                            camel_name,
                                            params,
                                            return_type: return_metadata,
                                            is_async,
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }

    if !found_state_struct {
        return Err(format!(
            "ACTIONABLE ERROR: state struct {} not found in {:?}",
            state_name, file_path
        ));
    }

    if !found_api_impl {
        return Err(format!(
            "ACTIONABLE ERROR: #[lera::api] impl for {} not found in {:?}",
            model_name, file_path
        ));
    }

    Ok(ParsedModel {
        model_name,
        state_name: state_name.to_string(),
        listener_name,
        default_state_fn: to_default_state_fn_name(state_name),
        methods,
        source_path: file_path.to_path_buf(),
    })
}

fn parse_method_parameters(
    sig: &syn::Signature,
    defaults: Option<&HashMap<String, Option<Expr>>>,
) -> Vec<ParsedParam> {
    let mut params = Vec::new();

    for input in &sig.inputs {
        match input {
            FnArg::Typed(pat_type) => {
                if let Pat::Ident(pat_ident) = &*pat_type.pat {
                    let param_name = pat_ident.ident.to_string();
                    if param_name == "self" {
                        continue;
                    }

                    let default =
                        defaults
                            .and_then(|map| map.get(&param_name))
                            .map(|entry| match entry {
                                Some(expr) => DefaultParamValue::ExplicitExpr(expr.clone()),
                                None => DefaultParamValue::Infer,
                            });

                    params.push(ParsedParam {
                        name: param_name,
                        ty: (*pat_type.ty).clone(),
                        default,
                    });
                }
            }
            FnArg::Receiver(_) => {}
        }
    }

    params
}

fn parse_return_metadata(sig: &syn::Signature) -> ParsedReturnType {
    match &sig.output {
        ReturnType::Default => ParsedReturnType {
            ty: None,
            uses_result: false,
        },
        ReturnType::Type(_, ty) => {
            if let Some((ok_ty, _err_ty)) = try_extract_result_types(ty) {
                let swift_ty = if is_unit_type(ok_ty) {
                    None
                } else {
                    Some(ok_ty.clone())
                };
                ParsedReturnType {
                    ty: swift_ty,
                    uses_result: true,
                }
            } else if is_unit_type(ty) {
                ParsedReturnType {
                    ty: None,
                    uses_result: false,
                }
            } else {
                ParsedReturnType {
                    ty: Some((**ty).clone()),
                    uses_result: false,
                }
            }
        }
    }
}

fn is_unit_type(ty: &Type) -> bool {
    matches!(ty, Type::Tuple(tuple) if tuple.elems.is_empty())
}

struct ModelAttrArgs {
    state_ty: Type,
}

impl Parse for ModelAttrArgs {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let key: syn::Ident = input.parse()?;
        if key != "state" {
            return Err(syn::Error::new(
                key.span(),
                "expected `state` argument, e.g. #[lera::model(state = MyState)]",
            ));
        }

        input.parse::<syn::Token![=]>()?;
        let state_ty: Type = input.parse()?;

        if input.peek(syn::Token![,]) {
            input.parse::<syn::Token![,]>()?;
            if !input.is_empty() {
                return Err(input.error("unexpected additional arguments"));
            }
        }

        Ok(Self { state_ty })
    }
}

pub fn type_path_generic_args(segment: &syn::PathSegment) -> Vec<&Type> {
    match &segment.arguments {
        syn::PathArguments::AngleBracketed(args) => args
            .args
            .iter()
            .filter_map(|arg| match arg {
                syn::GenericArgument::Type(ty) => Some(ty),
                _ => None,
            })
            .collect(),
        _ => Vec::new(),
    }
}

pub fn try_extract_result_types(ty: &Type) -> Option<(&Type, &Type)> {
    match ty {
        Type::Path(type_path) => {
            let segment = type_path.path.segments.last()?;
            if segment.ident == "Result" {
                let args = type_path_generic_args(segment);
                if args.len() >= 2 {
                    return Some((args[0], args[1]));
                }
            }
            None
        }
        Type::Group(group) => try_extract_result_types(&group.elem),
        Type::Paren(paren) => try_extract_result_types(&paren.elem),
        _ => None,
    }
}

pub fn type_to_string(ty: &Type) -> String {
    match ty {
        Type::Path(type_path) => type_path
            .path
            .segments
            .last()
            .map(|seg| seg.ident.to_string())
            .unwrap_or_else(|| "Unknown".to_string()),
        Type::Reference(type_ref) => {
            format!("&{}", type_to_string(&type_ref.elem))
        }
        Type::Tuple(tuple) if tuple.elems.is_empty() => "Unit".to_string(),
        Type::Tuple(tuple) => {
            let elems: Vec<String> = tuple.elems.iter().map(type_to_string).collect();
            format!("({})", elems.join(", "))
        }
        Type::Array(array) => format!("[{}; _]", type_to_string(&array.elem)),
        Type::Slice(slice) => format!("[{}]", type_to_string(&slice.elem)),
        _ => "Unknown".to_string(),
    }
}
