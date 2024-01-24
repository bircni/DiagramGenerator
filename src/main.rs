use anyhow::Context;
use itertools::Itertools;
use module::ModuleContext;
use serde_json::json;
use syn::{spanned::Spanned, Field, Visibility};
use tinytemplate::TinyTemplate;

use crate::structs::{StructContext, StructFieldContext};

mod module;
mod structs;

pub trait ToHtml {
    fn to_html(&self) -> String;
}

const HTML_TEMPLATE: &str = r#"
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <title>Diagram</title>
    <link rel="stylesheet" href="diagram.css">
</head>
<style>{ contents | style }</style>
<body>
    <h1>Diagram</h1>

    {contents}
</body>
</html>
"#;

fn main() {
    let path = std::env::args().nth(1).unwrap();

    let contents = parse_file_recursive(path).expect("Failed to parse file");

    let mut tt = TinyTemplate::new();
    tt.set_default_formatter(&tinytemplate::format_unescaped);
    tt.add_formatter("style", |_, string| {
        string.push_str(include_str!("style.css"));
        Ok(())
    });
    tt.add_template("html", HTML_TEMPLATE)
        .expect("Failed to add template");
    let html = tt
        .render("html", &json!({"contents": contents}))
        .expect("Failed to render template");

    std::fs::write("diagram.html", html).expect("Failed to write file");
}

fn parse_file_recursive<P: AsRef<std::path::Path>>(path: P) -> anyhow::Result<String> {
    let contents = std::fs::read_to_string(path.as_ref())
        .context(format!("Failed to read file {}", path.as_ref().display()))?;

    Ok(syn::parse_file(contents.as_str())
        .unwrap()
        .items
        .into_iter()
        .sorted_by(|a, b| match (a, b) {
            (syn::Item::Mod(_), _) => std::cmp::Ordering::Greater,
            (_, syn::Item::Mod(_)) => std::cmp::Ordering::Less,
            _ => std::cmp::Ordering::Equal,
        })
        .map(|i| traverse_ast(path.as_ref(), i))
        .join(""))
}

fn traverse_ast<P: AsRef<std::path::Path>>(path: P, ast: syn::Item) -> String {
    match ast {
        /*syn::Item::Impl(imp) => imp
        .items
        .into_iter()
        .map(|i| {
            format!(
                "{}::{}",
                imp.self_ty.span().source_text().unwrap(),
                match i {
                    syn::ImplItem::Const(c) => {
                        format!("{}: {}", c.ident, c.ty.span().source_text().unwrap())
                    }
                    syn::ImplItem::Fn(f) => f
                        .sig
                        .span()
                        .source_text()
                        .unwrap()
                        .trim_start_matches("fn ")
                        .to_string(),
                    syn::ImplItem::Type(t) => t.span().source_text().unwrap().to_string(),
                    _ => "".to_string(),
                }
            )
        })
        .join(""),*/
        syn::Item::Struct(s) => {
            let (public, private) = s
                .fields
                .clone()
                .into_iter()
                .enumerate()
                .partition::<Vec<(usize, Field)>, _>(|(_, f)| {
                    matches!(f.vis, Visibility::Public(_))
                });

            StructContext {
                name: format!(
                    "{}{}",
                    s.ident,
                    s.generics.span().source_text().unwrap_or_default()
                ),
                public_fields: public
                    .into_iter()
                    .map(|(i, f)| StructFieldContext {
                        name: f
                            .ident
                            .as_ref()
                            .map(|i| i.to_string())
                            .unwrap_or(format!("{}", i)),
                        type_: f.ty.span().source_text().unwrap().to_string(),
                    })
                    .collect(),
                private_fields: private
                    .into_iter()
                    .map(|(i, f)| structs::StructFieldContext {
                        name: f
                            .ident
                            .as_ref()
                            .map(|i| i.to_string())
                            .unwrap_or(format!("{}", i)),
                        type_: f.ty.span().source_text().unwrap().to_string(),
                    })
                    .collect(),
            }
            .to_html()
        }
        syn::Item::Mod(m) => ModuleContext {
            name: m.ident.to_string(),
            contents: if let Some((_, items)) = m.content {
                items
                    .into_iter()
                    .sorted_by(|a, b| match (a, b) {
                        (syn::Item::Mod(_), _) => std::cmp::Ordering::Greater,
                        (_, syn::Item::Mod(_)) => std::cmp::Ordering::Less,
                        _ => std::cmp::Ordering::Equal,
                    })
                    .map(|i| traverse_ast(path.as_ref(), i))
                    .collect()
            } else {
                parse_file_recursive(
                    path.as_ref()
                        .parent()
                        .unwrap()
                        .join(format!("{}.rs", m.ident)),
                )
                .or(parse_file_recursive(
                    path.as_ref()
                        .parent()
                        .unwrap()
                        .join(format!("{}/mod.rs", m.ident)),
                ))
                .expect("Failed to parse mod")
            },
        }
        .to_html(),
        _ => "".to_string(),
    }
}
