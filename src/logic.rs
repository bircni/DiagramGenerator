use anyhow::{Context, Error};
use itertools::Itertools;
use std::{cmp::Ordering, fs, path::Path};
use syn::{Field, Item, Visibility, parse_file, spanned::Spanned};

use crate::items::{
    ToHtml as _,
    module::ModContext,
    structs::{StructContext, StructFieldContext},
};

pub fn parse_file_recursive<P: AsRef<Path>>(path: P) -> anyhow::Result<String> {
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
