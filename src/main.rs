use anyhow::{Context, Error};
use itertools::Itertools;
use module::ModContext;
use serde_json::json;
use std::{cmp::Ordering, env, fs, path::Path};
use syn::{Field, Item, Visibility, parse_file, spanned::Spanned};
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

fn main() -> anyhow::Result<()> {
    let path = env::args().nth(1).context("No input file provided")?;

    let contents = parse_file_recursive(&path).context(format!("Failed to parse file: {path}"))?;

    let mut tt = TinyTemplate::new();
    tt.set_default_formatter(&tinytemplate::format_unescaped);
    tt.add_formatter("style", |_, string| {
        string.push_str(include_str!("style.css"));
        Ok(())
    });
    tt.add_template("html", HTML_TEMPLATE)
        .context("Failed to add template")?;
    let html = tt
        .render("html", &json!({"contents": contents}))
        .context("Failed to render template")?;

    fs::write("diagram.html", html).context("Failed to write file")?;

    Ok(())
}

fn parse_file_recursive<P: AsRef<Path>>(path: P) -> anyhow::Result<String> {
    let contents = fs::read_to_string(path.as_ref())
        .context(format!("Failed to read file {}", path.as_ref().display()))?;

    let parsed_file = parse_file(contents.as_str())?;
    let string_ret = parsed_file
        .items
        .into_iter()
        .sorted_by(|a, b| match (a, b) {
            (Item::Mod(_), _) => Ordering::Greater,
            (_, Item::Mod(_)) => Ordering::Less,
            _ => Ordering::Equal,
        })
        .map(|i| traverse_ast(path.as_ref(), i).expect("msg"))
        .join("");

    Ok(string_ret)
}

fn traverse_ast<P: AsRef<Path>>(path: P, ast: Item) -> anyhow::Result<String> {
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
        Item::Struct(s) => {
            let (public, private) = s
                .fields
                .clone()
                .into_iter()
                .enumerate()
                .partition::<Vec<(usize, Field)>, _>(|(_, f)| {
                    matches!(f.vis, Visibility::Public(_))
                });

            let public_fields = public
                .into_iter()
                .map(|(i, f)| StructFieldContext {
                    name: f
                        .ident
                        .as_ref()
                        .map_or_else(|| format!("{i}"), ToString::to_string),
                    type_: f
                        .ty
                        .span()
                        .source_text()
                        .expect("Could not get source_text"),
                })
                .collect();

            let private_fields = private
                .into_iter()
                .map(|(i, f)| StructFieldContext {
                    name: f
                        .ident
                        .as_ref()
                        .map_or_else(|| format!("{i}"), ToString::to_string),
                    type_: f
                        .ty
                        .span()
                        .source_text()
                        .expect("Could not get source_text"),
                })
                .collect();

            let context = StructContext {
                name: format!(
                    "{}{}",
                    s.ident,
                    s.generics.span().source_text().unwrap_or_default()
                ),
                public_fields,
                private_fields,
            };

            Ok(context.to_html())
        }
        Item::Mod(m) => {
            let contents: String = if let Some((_, items)) = m.content {
                let mut result = String::new();
                for item in items.into_iter().sorted_by(|a, b| match (a, b) {
                    (Item::Mod(_), _) => Ordering::Greater,
                    (_, Item::Mod(_)) => Ordering::Less,
                    _ => Ordering::Equal,
                }) {
                    let res = traverse_ast(path.as_ref(), item)?;
                    result.push_str(&res);
                }

                Ok::<String, Error>(result)
            } else {
                let file = parse_file_recursive(
                    path.as_ref()
                        .parent()
                        .context("Failed to get parent")?
                        .join(format!("{}.rs", m.ident)),
                )
                .or(parse_file_recursive(
                    path.as_ref()
                        .parent()
                        .context("Failed to get parent")?
                        .join(format!("{}/mod.rs", m.ident)),
                ))
                .context("Failed to parse mod")?;

                Ok(file)
            }?;

            let mod_context = ModContext {
                name: m.ident.to_string(),
                contents,
            };
            Ok(mod_context.to_html())
        }
        Item::Const(_)
        | Item::Enum(_)
        | Item::ExternCrate(_)
        | Item::Fn(_)
        | Item::ForeignMod(_)
        | Item::Impl(_)
        | Item::Macro(_)
        | Item::Static(_)
        | Item::Trait(_)
        | Item::TraitAlias(_)
        | Item::Type(_)
        | Item::Union(_)
        | Item::Use(_)
        | Item::Verbatim(_)
        | _ => Ok(String::new()),
    }
}
