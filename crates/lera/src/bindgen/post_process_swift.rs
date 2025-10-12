use askama::Template;
use quote::ToTokens;
use std::path::Path;
use syn::{Expr, ExprUnary, Type, TypePath, UnOp};

use super::post_process_shared::{
    DefaultParamValue, ParsedMethod, ParsedModel, ParsedReturnType, parse_lera_models,
    to_camel_case, type_path_generic_args,
};

#[derive(Debug, Clone)]
pub struct MethodParam {
    pub name: String,
    pub param_type: String,
    pub default_value: Option<String>,
}

#[derive(Debug, Clone)]
pub struct LeraModelInfo {
    pub model_name: String,
    pub state_name: String,
    pub listener_name: String,
    pub default_state_fn: String,
    pub samples_state_fn: String,
    pub enable_samples: bool,
    pub methods: Vec<String>,
}

#[derive(Template)]
#[template(path = "view_model.swift.jinja", escape = "none")]
struct ViewModelTemplate {
    models: Vec<LeraModelInfo>,
}

struct ReturnMetadata {
    swift_type: Option<String>,
    uses_throws: bool,
}

pub(crate) fn swift_transform(
    corpus: String,
    path_to_target_rust_crate: &Path,
) -> Result<String, String> {
    println!("🔮 Post processing Swift...");

    let parsed_models = parse_lera_models(path_to_target_rust_crate)?;
    println!("📝 Found {} LeraModel implementations", parsed_models.len());

    let models: Vec<LeraModelInfo> = parsed_models.iter().map(build_model_info).collect();

    for model in &models {
        println!(
            "   - {} with {} methods",
            model.model_name,
            model.methods.len()
        );
    }

    let template = ViewModelTemplate { models };
    let generated_swift = template
        .render()
        .map_err(|e| format!("Template rendering failed: {}", e))?;

    let result = format!("{}\n\n{}", corpus, generated_swift);

    println!("🔮 Post processing Swift done ✨");
    Ok(result)
}

fn build_model_info(model: &ParsedModel) -> LeraModelInfo {
    let methods = model
        .methods
        .iter()
        .map(|method| build_method(method, model))
        .collect();

    LeraModelInfo {
        model_name: model.model_name.clone(),
        state_name: model.state_name.clone(),
        listener_name: model.listener_name.clone(),
        default_state_fn: model.default_state_fn.clone(),
        samples_state_fn: model.samples_state_fn.clone(),
        enable_samples: model.enable_samples,
        methods,
    }
}

fn build_method(method: &ParsedMethod, model: &ParsedModel) -> String {
    let parameters = method_params_to_swift(method, model);
    let camel_params: Vec<String> = parameters
        .iter()
        .map(|param| to_camel_case(&param.name))
        .collect();

    let return_metadata = convert_return_metadata(&method.return_type);
    let is_async = method.is_async;

    let param_declarations: Vec<String> = parameters
        .iter()
        .zip(&camel_params)
        .enumerate()
        .map(|(idx, (param, camel_name))| {
            let suffix = if idx + 1 == parameters.len() { "" } else { "," };
            let default_part = param
                .default_value
                .as_ref()
                .map(|v| format!(" = {}", v))
                .unwrap_or_default();
            format!(
                "\n\t\t{}: {}{}{}",
                camel_name, param.param_type, default_part, suffix
            )
        })
        .collect();

    let param_names: Vec<String> = camel_params
        .iter()
        .enumerate()
        .map(|(idx, camel_name)| {
            let suffix = if idx + 1 == camel_params.len() {
                ""
            } else {
                ","
            };
            format!("\n\t\t\t{}: {}{}", camel_name, camel_name, suffix)
        })
        .collect();

    let param_part = if param_declarations.is_empty() {
        "()".to_string()
    } else {
        format!("({}\n\t)", param_declarations.join(""))
    };

    let async_keyword = if is_async { " async" } else { "" };
    let throws_keyword = if return_metadata.uses_throws {
        " throws"
    } else {
        ""
    };
    let return_part = return_metadata
        .swift_type
        .as_ref()
        .map(|t| format!(" -> {}", t))
        .unwrap_or_default();

    let call_params = if param_names.is_empty() {
        "()".to_string()
    } else {
        format!("({}\n\t\t)", param_names.join(""))
    };

    let mut call_prefix_parts: Vec<&str> = Vec::new();
    if return_metadata.uses_throws {
        call_prefix_parts.push("try");
    }
    if is_async {
        call_prefix_parts.push("await");
    }
    let call_prefix = if call_prefix_parts.is_empty() {
        String::new()
    } else {
        format!("{} ", call_prefix_parts.join(" "))
    };

    format!(
        "\tpublic func {}{}{}{}{} {{\n\t\t{}model.{}{}\n\t}}",
        method.camel_name,
        param_part,
        async_keyword,
        throws_keyword,
        return_part,
        call_prefix,
        method.camel_name,
        call_params
    )
}

fn method_params_to_swift(method: &ParsedMethod, model: &ParsedModel) -> Vec<MethodParam> {
    method
        .params
        .iter()
        .map(|param| {
            let swift_type = swift_type_from_syn_type(&param.ty);
            let default_value = match param.default.as_ref() {
                Some(DefaultParamValue::ExplicitExpr(expr)) => {
                    default_expr_to_swift(expr, &swift_type).or_else(|| {
                        println!(
                            "⚠️  Unsupported default expression `{}` for `{}` in method `{}` ({})",
                            expr.to_token_stream(),
                            param.name,
                            method.rust_name,
                            model.source_path.display()
                        );
                        None
                    })
                }
                Some(DefaultParamValue::Infer) => {
                    infer_default_for_swift_type(&swift_type).or_else(|| {
                        println!(
                            "⚠️  Unable to infer default for parameter `{}` of type `{}` in method `{}` ({})",
                            param.name,
                            swift_type,
                            method.rust_name,
                            model.source_path.display()
                        );
                        None
                    })
                }
                None => None,
            };

            MethodParam {
                name: param.name.clone(),
                param_type: swift_type,
                default_value,
            }
        })
        .collect()
}

fn convert_return_metadata(return_type: &ParsedReturnType) -> ReturnMetadata {
    let swift_type = return_type.ty.as_ref().map(swift_type_from_syn_type);
    ReturnMetadata {
        swift_type,
        uses_throws: return_type.uses_result,
    }
}

fn swift_type_from_syn_type(ty: &Type) -> String {
    match ty {
        Type::Path(type_path) => swift_type_from_type_path(type_path),
        Type::Reference(type_ref) => {
            if let Type::Slice(slice) = &*type_ref.elem {
                if is_u8_slice(slice) {
                    return "Data".to_string();
                }
                let element = swift_type_from_syn_type(&slice.elem);
                return format!("Array<{}>", element);
            }
            swift_type_from_syn_type(&type_ref.elem)
        }
        Type::Slice(slice) => {
            if is_u8_slice(slice) {
                return "Array<UInt8>".to_string();
            }
            let element = swift_type_from_syn_type(&slice.elem);
            format!("Array<{}>", element)
        }
        Type::Array(array) => {
            let element = swift_type_from_syn_type(&array.elem);
            format!("Array<{}>", element)
        }
        Type::Tuple(tuple) => {
            if tuple.elems.is_empty() {
                "Void".to_string()
            } else {
                let elems: Vec<String> = tuple.elems.iter().map(swift_type_from_syn_type).collect();
                format!("({})", elems.join(", "))
            }
        }
        Type::Paren(paren) => swift_type_from_syn_type(&paren.elem),
        Type::Group(group) => swift_type_from_syn_type(&group.elem),
        _ => type_to_string(ty),
    }
}

fn swift_type_from_type_path(type_path: &TypePath) -> String {
    let ident = match type_path.path.segments.last() {
        Some(segment) => segment,
        None => return "Unknown".to_string(),
    };

    let ident_str = ident.ident.to_string();

    match ident_str.as_str() {
        "bool" => "Bool".to_string(),
        "u8" => "UInt8".to_string(),
        "u16" => "UInt16".to_string(),
        "u32" => "UInt32".to_string(),
        "u64" => "UInt64".to_string(),
        "usize" => "UInt".to_string(),
        "i8" => "Int8".to_string(),
        "i16" => "Int16".to_string(),
        "i32" => "Int32".to_string(),
        "i64" => "Int64".to_string(),
        "isize" => "Int".to_string(),
        "f32" => "Float".to_string(),
        "f64" => "Double".to_string(),
        "String" | "str" => "String".to_string(),
        "Vec" | "VecDeque" => {
            let inner = type_path_generic_args(ident).first().cloned();
            let inner = inner
                .map(swift_type_from_syn_type)
                .unwrap_or_else(|| ident_str.clone());
            format!("Array<{}>", inner)
        }
        "HashMap" | "BTreeMap" => {
            let args = type_path_generic_args(ident);
            if args.len() >= 2 {
                let key = swift_type_from_syn_type(args[0]);
                let value = swift_type_from_syn_type(args[1]);
                format!("Dictionary<{}, {}>", key, value)
            } else {
                ident_str
            }
        }
        "HashSet" | "BTreeSet" => {
            let inner = type_path_generic_args(ident).first().cloned();
            let inner = inner
                .map(swift_type_from_syn_type)
                .unwrap_or_else(|| ident_str.clone());
            format!("Set<{}>", inner)
        }
        "Option" => {
            let inner = type_path_generic_args(ident).first().cloned();
            let inner = inner
                .map(swift_type_from_syn_type)
                .unwrap_or_else(|| ident_str.clone());
            format!("{}?", inner)
        }
        "Result" => {
            let args = type_path_generic_args(ident);
            if args.len() >= 2 {
                let ok = swift_type_from_syn_type(args[0]);
                let err = swift_type_from_syn_type(args[1]);
                format!("Result<{}, {}>", ok, err)
            } else {
                ident_str
            }
        }
        "Arc" | "Rc" | "Box" => {
            let inner = type_path_generic_args(ident).first().cloned();
            inner.map_or(ident_str, swift_type_from_syn_type)
        }
        _ => ident_str,
    }
}

fn is_u8_slice(slice: &syn::TypeSlice) -> bool {
    matches!(&*slice.elem, Type::Path(path) if path.path.is_ident("u8"))
}

fn default_expr_to_swift(expr: &Expr, swift_type: &str) -> Option<String> {
    match expr {
        Expr::Lit(expr_lit) => match &expr_lit.lit {
            syn::Lit::Bool(lit) => Some(lit.value.to_string()),
            syn::Lit::Int(lit) => Some(lit.base10_digits().to_string()),
            syn::Lit::Float(lit) => Some(lit.to_string()),
            syn::Lit::Str(lit) => Some(format!("\"{}\"", escape_swift_string(&lit.value()))),
            _ => None,
        },
        Expr::Unary(ExprUnary {
            op: UnOp::Neg(_),
            expr: inner,
            ..
        }) => default_expr_to_swift(inner, swift_type).map(|value| format!("-{}", value)),
        Expr::Path(expr_path) => {
            let mut segments = expr_path.path.segments.iter();
            if let Some(first) = segments.next() {
                if first.ident == "None" && segments.next().is_none() {
                    return Some("nil".to_string());
                }
            }
            None
        }
        Expr::Array(expr_array) => {
            if expr_array.elems.is_empty() {
                if swift_type == "Data" {
                    Some("Data()".to_string())
                } else {
                    Some("[]".to_string())
                }
            } else {
                let mut elements = Vec::new();
                for elem in expr_array.elems.iter() {
                    if let Some(value) = default_expr_to_swift(elem, swift_type) {
                        elements.push(value);
                    } else {
                        return None;
                    }
                }
                if swift_type == "Data" {
                    Some(format!("Data([{}])", elements.join(", ")))
                } else {
                    Some(format!("[{}]", elements.join(", ")))
                }
            }
        }
        _ => None,
    }
}

fn infer_default_for_swift_type(swift_type: &str) -> Option<String> {
    match swift_type {
        "Bool" => Some("false".to_string()),
        "Int8" | "Int16" | "Int32" | "Int64" | "Int" | "UInt8" | "UInt16" | "UInt32" | "UInt64"
        | "UInt" => Some("0".to_string()),
        "Float" => Some("0.0".to_string()),
        "Double" => Some("0.0".to_string()),
        "String" => Some("\"\"".to_string()),
        "Data" => Some("Data()".to_string()),
        _ => {
            if swift_type.ends_with('?') {
                Some("nil".to_string())
            } else if swift_type.starts_with("Array<") || swift_type.starts_with('[') {
                Some("[]".to_string())
            } else if swift_type.starts_with("Dictionary<") {
                Some("[:]".to_string())
            } else if swift_type.starts_with("Set<") {
                Some("Set()".to_string())
            } else {
                None
            }
        }
    }
}

fn escape_swift_string(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

fn type_to_string(ty: &Type) -> String {
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
        Type::Tuple(tuple) if tuple.elems.is_empty() => "Void".to_string(),
        Type::Tuple(tuple) => {
            let elems: Vec<String> = tuple.elems.iter().map(type_to_string).collect();
            format!("({})", elems.join(", "))
        }
        Type::Array(array) => format!("[{}; _]", type_to_string(&array.elem)),
        Type::Slice(slice) => format!("[{}]", type_to_string(&slice.elem)),
        _ => "Unknown".to_string(),
    }
}
