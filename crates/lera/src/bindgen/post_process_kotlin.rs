use askama::Template;
use quote::ToTokens;
use std::path::Path;
use syn::{Expr, ExprUnary, Type, TypePath, UnOp};

use super::post_process_shared::{
    DefaultParamValue, ParsedMethod, ParsedModel, ParsedReturnType, parse_lera_models,
    to_camel_case, type_path_generic_args,
};

#[derive(Debug, Clone)]
struct KotlinMethodParam {
    name: String,
    param_type: String,
    default_value: Option<String>,
}

#[derive(Debug, Clone)]
struct KotlinModelInfo {
    model_name: String,
    state_name: String,
    listener_name: String,
    default_state_fn: String,
    samples_state_fn: String,
    methods: Vec<String>,
}

#[derive(Template)]
#[template(path = "view_model.kt.jinja", escape = "none")]
struct KotlinViewModelTemplate {
    models: Vec<KotlinModelInfo>,
}

struct ReturnMetadata {
    kotlin_type: Option<String>,
}

pub(crate) fn kotlin_transform(
    corpus: String,
    path_to_target_rust_crate: &Path,
) -> Result<String, String> {
    println!("🔮 Post processing Kotlin...");

    let parsed_models = parse_lera_models(path_to_target_rust_crate)?;
    println!("📝 Found {} LeraModel implementations", parsed_models.len());

    let models: Vec<KotlinModelInfo> = parsed_models
        .iter()
        .map(build_model_info)
        .collect::<Result<_, _>>()?;

    for model in &models {
        println!(
            "   - {} with {} methods",
            model.model_name,
            model.methods.len()
        );
    }

    let template = KotlinViewModelTemplate { models };
    let generated_kotlin = template
        .render()
        .map_err(|e| format!("Template rendering failed: {}", e))?;

    let mut result = corpus;
    if !result.contains("kotlinx.coroutines.flow.StateFlow") {
        let imports = "import androidx.lifecycle.ViewModel\n".to_string()
            + "import kotlinx.coroutines.flow.MutableStateFlow\n"
            + "import kotlinx.coroutines.flow.StateFlow\n"
            + "import kotlinx.coroutines.flow.asStateFlow\n";

        if let Some(pkg_pos) = result.find("\npackage ") {
            let pkg_line_start = pkg_pos + 1;
            if let Some(line_end) = result[pkg_line_start..].find('\n') {
                let insert_pos = pkg_line_start + line_end + 1;
                result.insert_str(insert_pos, &format!("{}\n", imports));
            }
        }
    }

    result.push_str("\n\n");
    result.push_str(&generated_kotlin);

    println!("🔮 Post processing Kotlin done ✨");
    Ok(result)
}

fn build_model_info(model: &ParsedModel) -> Result<KotlinModelInfo, String> {
    let methods = model
        .methods
        .iter()
        .map(|method| build_method(method, model))
        .collect::<Result<Vec<_>, _>>()?;

    Ok(KotlinModelInfo {
        model_name: model.model_name.clone(),
        state_name: model.state_name.clone(),
        listener_name: model.listener_name.clone(),
        default_state_fn: model.default_state_fn.clone(),
        samples_state_fn: model.samples_state_fn.clone(),
        methods,
    })
}

fn build_method(method: &ParsedMethod, model: &ParsedModel) -> Result<String, String> {
    let parameters = method_params_to_kotlin(method, model)?;
    let camel_params: Vec<String> = parameters
        .iter()
        .map(|param| to_camel_case(&param.name))
        .collect();

    let return_metadata = convert_return_metadata(&method.return_type);
    let suspend_keyword = if method.is_async { "suspend " } else { "" };

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
                "        {}: {}{}{}",
                camel_name, param.param_type, default_part, suffix
            )
        })
        .collect();

    let param_part = if param_declarations.is_empty() {
        "()".to_string()
    } else {
        format!("(\n{}\n    )", param_declarations.join("\n"))
    };

    let call_args: Vec<String> = camel_params
        .iter()
        .enumerate()
        .map(|(idx, camel_name)| {
            let suffix = if idx + 1 == camel_params.len() {
                ""
            } else {
                ","
            };
            format!("            {}{}", camel_name, suffix)
        })
        .collect();

    let call_params = if call_args.is_empty() {
        "()".to_string()
    } else {
        format!("(\n{}\n        )", call_args.join("\n"))
    };

    let return_part = return_metadata
        .kotlin_type
        .as_ref()
        .map(|t| format!(": {}", t))
        .unwrap_or_default();

    let call_prefix = if return_metadata.kotlin_type.is_some() {
        "return "
    } else {
        ""
    };

    Ok(format!(
        "{}fun {}{}{} {{\n        {}model.{}{}\n    }}",
        suspend_keyword,
        method.camel_name,
        param_part,
        return_part,
        call_prefix,
        method.camel_name,
        call_params
    ))
}

fn method_params_to_kotlin(
    method: &ParsedMethod,
    model: &ParsedModel,
) -> Result<Vec<KotlinMethodParam>, String> {
    method
        .params
        .iter()
        .map(|param| {
            let kotlin_type = kotlin_type_from_syn_type(&param.ty);
            let default_value = match param.default.as_ref() {
                Some(DefaultParamValue::ExplicitExpr(expr)) => {
                    default_expr_to_kotlin(expr, &kotlin_type).or_else(|| {
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
                Some(DefaultParamValue::Infer) => infer_default_for_kotlin_type(&kotlin_type)
                    .or_else(|| {
                        println!(
                            "⚠️  Unable to infer default for parameter `{}` of type `{}` in method `{}` ({})",
                            param.name,
                            kotlin_type,
                            method.rust_name,
                            model.source_path.display()
                        );
                        None
                    }),
                None => None,
            };

            Ok(KotlinMethodParam {
                name: param.name.clone(),
                param_type: kotlin_type,
                default_value,
            })
        })
        .collect()
}

fn convert_return_metadata(return_type: &ParsedReturnType) -> ReturnMetadata {
    let kotlin_type = return_type.ty.as_ref().map(kotlin_type_from_syn_type);
    ReturnMetadata { kotlin_type }
}

fn kotlin_type_from_syn_type(ty: &Type) -> String {
    match ty {
        Type::Path(type_path) => kotlin_type_from_type_path(type_path),
        Type::Reference(type_ref) => {
            if let Type::Slice(slice) = &*type_ref.elem {
                if is_u8_slice(slice) {
                    return "ByteArray".to_string();
                }
                let element = kotlin_type_from_syn_type(&slice.elem);
                return format!("List<{}>", element);
            }
            kotlin_type_from_syn_type(&type_ref.elem)
        }
        Type::Slice(slice) => {
            if is_u8_slice(slice) {
                return "ByteArray".to_string();
            }
            let element = kotlin_type_from_syn_type(&slice.elem);
            format!("List<{}>", element)
        }
        Type::Array(array) => {
            let element = kotlin_type_from_syn_type(&array.elem);
            format!("List<{}>", element)
        }
        Type::Tuple(tuple) => {
            if tuple.elems.is_empty() {
                "Unit".to_string()
            } else {
                let elems: Vec<String> =
                    tuple.elems.iter().map(kotlin_type_from_syn_type).collect();
                format!("Pair<{}>", elems.join(", "))
            }
        }
        Type::Paren(paren) => kotlin_type_from_syn_type(&paren.elem),
        Type::Group(group) => kotlin_type_from_syn_type(&group.elem),
        _ => type_to_string(ty),
    }
}

fn kotlin_type_from_type_path(type_path: &TypePath) -> String {
    let ident = match type_path.path.segments.last() {
        Some(segment) => segment,
        None => return "Unknown".to_string(),
    };

    let ident_str = ident.ident.to_string();

    match ident_str.as_str() {
        "bool" => "Boolean".to_string(),
        "u8" => "UByte".to_string(),
        "u16" => "UShort".to_string(),
        "u32" => "UInt".to_string(),
        "u64" => "ULong".to_string(),
        "usize" => "ULong".to_string(),
        "i8" => "Byte".to_string(),
        "i16" => "Short".to_string(),
        "i32" => "Int".to_string(),
        "i64" => "Long".to_string(),
        "isize" => "Long".to_string(),
        "f32" => "Float".to_string(),
        "f64" => "Double".to_string(),
        "String" | "str" => "String".to_string(),
        "Vec" | "VecDeque" => {
            let inner = type_path_generic_args(ident).first().cloned();
            let inner = inner
                .map(kotlin_type_from_syn_type)
                .unwrap_or_else(|| ident_str.clone());
            if inner == "UByte" {
                "ByteArray".to_string()
            } else {
                format!("List<{}>", inner)
            }
        }
        "HashMap" | "BTreeMap" => {
            let args = type_path_generic_args(ident);
            if args.len() >= 2 {
                let key = kotlin_type_from_syn_type(args[0]);
                let value = kotlin_type_from_syn_type(args[1]);
                format!("Map<{}, {}>", key, value)
            } else {
                ident_str
            }
        }
        "HashSet" | "BTreeSet" => {
            let inner = type_path_generic_args(ident).first().cloned();
            let inner = inner
                .map(kotlin_type_from_syn_type)
                .unwrap_or_else(|| ident_str.clone());
            format!("Set<{}>", inner)
        }
        "Option" => {
            let inner = type_path_generic_args(ident).first().cloned();
            let inner = inner
                .map(kotlin_type_from_syn_type)
                .unwrap_or_else(|| ident_str.clone());
            format!("{}?", inner)
        }
        "Result" => {
            let args = type_path_generic_args(ident);
            if args.len() >= 2 {
                let ok = kotlin_type_from_syn_type(args[0]);
                let err = kotlin_type_from_syn_type(args[1]);
                format!("Result<{}, {}>", ok, err)
            } else {
                ident_str
            }
        }
        "Arc" | "Rc" | "Box" => {
            let inner = type_path_generic_args(ident).first().cloned();
            inner.map_or(ident_str, kotlin_type_from_syn_type)
        }
        _ => ident_str,
    }
}

fn is_u8_slice(slice: &syn::TypeSlice) -> bool {
    matches!(&*slice.elem, Type::Path(path) if path.path.is_ident("u8"))
}

fn default_expr_to_kotlin(expr: &Expr, kotlin_type: &str) -> Option<String> {
    match expr {
        Expr::Lit(expr_lit) => match &expr_lit.lit {
            syn::Lit::Bool(lit) => Some(lit.value.to_string()),
            syn::Lit::Int(lit) => {
                let value = lit.base10_digits();
                Some(apply_numeric_suffix(value, kotlin_type))
            }
            syn::Lit::Float(lit) => Some(apply_float_suffix(lit.to_string(), kotlin_type)),
            syn::Lit::Str(lit) => Some(format!("\"{}\"", escape_kotlin_string(&lit.value()))),
            _ => None,
        },
        Expr::Unary(ExprUnary {
            op: UnOp::Neg(_),
            expr: inner,
            ..
        }) => default_expr_to_kotlin(inner, kotlin_type).map(|value| match kotlin_type {
            "Byte" => value
                .strip_suffix(".toByte()")
                .map(|num| format!("(-{}).toByte()", num))
                .unwrap_or_else(|| format!("-{}", value)),
            "Short" => value
                .strip_suffix(".toShort()")
                .map(|num| format!("(-{}).toShort()", num))
                .unwrap_or_else(|| format!("-{}", value)),
            _ => format!("-{}", value),
        }),
        Expr::Path(expr_path) => {
            let mut segments = expr_path.path.segments.iter();
            if let Some(first) = segments.next() {
                if first.ident == "None" && segments.next().is_none() {
                    return Some("null".to_string());
                }
            }
            None
        }
        Expr::Array(expr_array) => {
            if expr_array.elems.is_empty() {
                match kotlin_type {
                    "ByteArray" => Some("byteArrayOf()".to_string()),
                    ty if ty.starts_with("List<") => Some("listOf()".to_string()),
                    ty if ty.starts_with("Set<") => Some("setOf()".to_string()),
                    _ => Some("listOf()".to_string()),
                }
            } else {
                None
            }
        }
        _ => None,
    }
}

fn infer_default_for_kotlin_type(kotlin_type: &str) -> Option<String> {
    match kotlin_type {
        "Boolean" => Some("false".to_string()),
        "Byte" => Some("0.toByte()".to_string()),
        "Short" => Some("0.toShort()".to_string()),
        "Int" => Some("0".to_string()),
        "Long" => Some("0L".to_string()),
        "UByte" => Some("0u.toUByte()".to_string()),
        "UShort" => Some("0u.toUShort()".to_string()),
        "UInt" => Some("0u".to_string()),
        "ULong" => Some("0UL".to_string()),
        "Float" => Some("0.0f".to_string()),
        "Double" => Some("0.0".to_string()),
        "String" => Some("\"\"".to_string()),
        "ByteArray" => Some("byteArrayOf()".to_string()),
        ty if ty.ends_with('?') => Some("null".to_string()),
        ty if ty.starts_with("List<") => Some("listOf()".to_string()),
        ty if ty.starts_with("Map<") => Some("mapOf()".to_string()),
        ty if ty.starts_with("Set<") => Some("setOf()".to_string()),
        _ => None,
    }
}

fn apply_numeric_suffix(value: &str, kotlin_type: &str) -> String {
    match kotlin_type {
        "Byte" => format!("{}.toByte()", value),
        "Short" => format!("{}.toShort()", value),
        "Int" => value.to_string(),
        "Long" => format!("{}L", value),
        "UByte" => format!("{}u.toUByte()", value),
        "UShort" => format!("{}u.toUShort()", value),
        "UInt" => format!("{}u", value),
        "ULong" => format!("{}UL", value),
        _ => value.to_string(),
    }
}

fn apply_float_suffix(value: String, kotlin_type: &str) -> String {
    match kotlin_type {
        "Float" => format!("{}f", value),
        _ => value,
    }
}

fn escape_kotlin_string(value: &str) -> String {
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
