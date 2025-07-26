use anyhow::Context;
use itertools::Itertools;
use std::{fs, path::Path};
use syn::{Field, Item, Visibility, parse_file, spanned::Spanned};

use crate::items::{
    ToHtml as _,
    enums::{EnumContext, EnumVariantContext},
    functions::FunctionContext,
    impl_blocks::ImplContext,
    module::ModContext,
    structs::{StructContext, StructFieldContext},
};

pub(crate) fn parse_file_recursive<P: AsRef<Path>>(
    path: P,
    include_tests: bool,
) -> anyhow::Result<String> {
    let contents = fs::read_to_string(path.as_ref())
        .context(format!("Failed to read file {}", path.as_ref().display()))?;

    let parsed_file = parse_file(contents.as_str())?;
    let string_ret =
        organize_and_render_items(path.as_ref(), parsed_file.items, include_tests, false)?;

    Ok(string_ret)
}

/// Helper function for parsing files with test context
fn parse_file_recursive_with_context<P: AsRef<Path>>(
    path: P,
    include_tests: bool,
    in_test_context: bool,
) -> anyhow::Result<String> {
    let contents = fs::read_to_string(path.as_ref())
        .context(format!("Failed to read file {}", path.as_ref().display()))?;

    let parsed_file = parse_file(contents.as_str())?;
    let string_ret = organize_and_render_items(
        path.as_ref(),
        parsed_file.items,
        include_tests,
        in_test_context,
    )?;

    Ok(string_ret)
}

/// Organize items by type and render them in a structured way
pub(crate) fn organize_and_render_items<P: AsRef<Path>>(
    path: P,
    items: Vec<Item>,
    include_tests: bool,
    in_test_context: bool,
) -> anyhow::Result<String> {
    let mut structs = Vec::new();
    let mut enums = Vec::new();
    let mut functions = Vec::new();
    let mut impls = Vec::new();
    let mut modules = Vec::new();
    let mut other_items = Vec::new();

    // Group items by type
    for item in items {
        match &item {
            Item::Struct(_) => structs.push(item),
            Item::Enum(_) => enums.push(item),
            Item::Fn(_) => functions.push(item),
            Item::Impl(_) => impls.push(item),
            Item::Mod(_) => modules.push(item),
            Item::Const(_)
            | Item::ExternCrate(_)
            | Item::ForeignMod(_)
            | Item::Macro(_)
            | Item::Static(_)
            | Item::Trait(_)
            | Item::TraitAlias(_)
            | Item::Type(_)
            | Item::Union(_)
            | Item::Use(_)
            | Item::Verbatim(_)
            | _ => other_items.push(item),
        }
    }

    let mut result = String::new();

    // Helper function to create a section with items
    let create_section =
        |section_name: &str, items: Vec<Item>, grid_class: &str| -> anyhow::Result<String> {
            if items.is_empty() {
                return Ok(String::new());
            }

            let mut section_html = format!(
                r#"<div class="item-section">
    <div class="item-section-header">{section_name}</div>
    <div class="{grid_class}">
"#
            );

            for item in items {
                let rendered = traverse_ast(path.as_ref(), item, include_tests, in_test_context)?;
                section_html.push_str(&rendered);
            }

            section_html.push_str("    </div>\n</div>\n\n");
            Ok(section_html)
        };

    // Render sections in organized order
    result.push_str(&create_section("Structs", structs, "structs-grid")?);
    result.push_str(&create_section("Enums", enums, "enums-grid")?);
    result.push_str(&create_section("Functions", functions, "functions-grid")?);
    result.push_str(&create_section(
        "Implementations",
        impls,
        "impl-blocks-grid",
    )?);

    // Modules get special treatment - they don't need a section wrapper
    for item in modules.into_iter().chain(other_items) {
        let rendered = traverse_ast(path.as_ref(), item, include_tests, in_test_context)?;
        result.push_str(&rendered);
    }

    Ok(result)
}

/// Check if an attribute list contains test-related attributes
fn has_test_attributes(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(|attr| {
        attr.meta.require_path_only().map_or_else(
            |_| {
                attr.meta.require_list().is_ok_and(|meta| {
                    let path_str = meta
                        .path
                        .get_ident()
                        .map(ToString::to_string)
                        .unwrap_or_default();
                    if path_str == "cfg" {
                        // Check if it's cfg(test)
                        syn::parse2::<syn::Ident>(meta.tokens.clone())
                            .is_ok_and(|tokens| tokens == "test")
                    } else {
                        false
                    }
                })
            },
            |meta| {
                let path_str = meta
                    .get_ident()
                    .map(ToString::to_string)
                    .unwrap_or_default();
                path_str == "test"
            },
        )
    })
}

/// Check if a module is a test module (either named "tests" or has #[cfg(test)])
fn is_test_module(module: &syn::ItemMod) -> bool {
    // Check if module has #[cfg(test)] attribute
    if has_test_attributes(&module.attrs) {
        return true;
    }

    // Check if module name is "tests" (common convention)
    module.ident == "tests"
}

/// Check if we're in a test context (inside a test module or function has test attributes)
fn should_exclude_from_tests(
    attrs: &[syn::Attribute],
    in_test_context: bool,
    include_tests: bool,
) -> bool {
    if include_tests {
        return false;
    }

    // Exclude if we're in a test context (inside a test module)
    if in_test_context {
        return true;
    }

    // Exclude if the item itself has test attributes
    has_test_attributes(attrs)
}

#[expect(
    clippy::too_many_lines,
    reason = "This function handles multiple item types which naturally makes it long"
)]
fn traverse_ast<P: AsRef<Path>>(
    path: P,
    ast: Item,
    include_tests: bool,
    in_test_context: bool,
) -> anyhow::Result<String> {
    match ast {
        Item::Impl(imp) => {
            let functions: Vec<String> = imp
                .items
                .into_iter()
                .filter_map(|item| match item {
                    syn::ImplItem::Fn(f) => {
                        if should_exclude_from_tests(&f.attrs, in_test_context, include_tests) {
                            None
                        } else {
                            let context = FunctionContext::new(&f.sig, &f.vis);
                            Some(context.to_html())
                        }
                    }
                    syn::ImplItem::Const(_)
                    | syn::ImplItem::Type(_)
                    | syn::ImplItem::Macro(_)
                    | syn::ImplItem::Verbatim(_)
                    | _ => None,
                })
                .filter_map(anyhow::Result::ok)
                .collect();

            // Only create impl block if it has functions to display
            if functions.is_empty() {
                return Ok(String::new());
            }

            let target_type = imp
                .self_ty
                .span()
                .source_text()
                .expect("Could not get source_text");

            let trait_name = imp.trait_.as_ref().map(|(_, path, _)| {
                path.span()
                    .source_text()
                    .expect("Could not get source_text")
            });

            let generics = if imp.generics.params.is_empty() {
                None
            } else {
                Some(
                    imp.generics
                        .span()
                        .source_text()
                        .expect("Could not get source_text"),
                )
            };

            let context = ImplContext {
                target_type,
                trait_name,
                generics,
                functions,
            };

            Ok(context.to_html()?)
        }
        Item::Struct(s) => {
            // Check if this struct should be excluded from tests
            if should_exclude_from_tests(&s.attrs, in_test_context, include_tests) {
                return Ok(String::new());
            }

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

            Ok(context.to_html()?)
        }
        Item::Enum(e) => {
            // Check if this enum should be excluded from tests
            if should_exclude_from_tests(&e.attrs, in_test_context, include_tests) {
                return Ok(String::new());
            }

            let variants = e
                .variants
                .into_iter()
                .map(|v| {
                    let data = match &v.fields {
                        syn::Fields::Named(fields) => {
                            let field_list = fields
                                .named
                                .iter()
                                .map(|f| {
                                    format!(
                                        "{}: {}",
                                        f.ident.as_ref().expect("Named field should have ident"),
                                        f.ty.span()
                                            .source_text()
                                            .expect("Could not get source_text")
                                    )
                                })
                                .join(", ");
                            Some(format!("{{ {field_list} }}"))
                        }
                        syn::Fields::Unnamed(fields) => {
                            let field_list = fields
                                .unnamed
                                .iter()
                                .map(|f| {
                                    f.ty.span()
                                        .source_text()
                                        .expect("Could not get source_text")
                                })
                                .join(", ");
                            Some(format!("({field_list})"))
                        }
                        syn::Fields::Unit => None,
                    };

                    EnumVariantContext {
                        name: v.ident.to_string(),
                        data,
                    }
                })
                .collect();

            let context = EnumContext {
                name: format!(
                    "{}{}",
                    e.ident,
                    e.generics.span().source_text().unwrap_or_default()
                ),
                variants,
            };

            Ok(context.to_html()?)
        }
        Item::Fn(f) => {
            // Check if this function should be excluded from tests
            if should_exclude_from_tests(&f.attrs, in_test_context, include_tests) {
                return Ok(String::new());
            }

            let context = FunctionContext::new(&f.sig, &f.vis);
            Ok(context.to_html()?)
        }
        Item::Mod(m) => {
            // Check if this module should be excluded (test modules)
            if !include_tests && is_test_module(&m) {
                return Ok(String::new());
            }

            // Determine if we're entering a test context
            let entering_test_context = in_test_context || is_test_module(&m);

            let contents: String = if let Some((_, items)) = m.content {
                // Use the same organized approach for module contents
                organize_and_render_items(
                    path.as_ref(),
                    items,
                    include_tests,
                    entering_test_context,
                )?
            } else {
                parse_file_recursive_with_context(
                    path.as_ref()
                        .parent()
                        .context("Failed to get parent")?
                        .join(format!("{}.rs", m.ident)),
                    include_tests,
                    entering_test_context,
                )
                .or(parse_file_recursive_with_context(
                    path.as_ref()
                        .parent()
                        .context("Failed to get parent")?
                        .join(format!("{}/mod.rs", m.ident)),
                    include_tests,
                    entering_test_context,
                ))
                .context("Failed to parse mod")?
            };

            // Don't render empty modules
            if contents.trim().is_empty() {
                return Ok(String::new());
            }

            let mod_context = ModContext {
                name: m.ident.to_string(),
                contents,
            };
            Ok(mod_context.to_html()?)
        }
        Item::Const(_)
        | Item::ExternCrate(_)
        | Item::ForeignMod(_)
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
