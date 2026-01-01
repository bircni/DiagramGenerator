use anyhow::Context as _;
use quote::ToTokens as _;
use std::collections::HashMap;
pub struct ItemVisitor<'a, V> {
    current_dir: std::path::PathBuf,
    v: &'a mut V,
    include_tests: bool,
    pending_impls: HashMap<String, Vec<syn::ItemImpl>>,
}

impl<'a, V: Visualizer> ItemVisitor<'a, V> {
    pub fn visit_file(
        file: impl AsRef<std::path::Path>,
        visualizer: &'a mut V,
        include_tests: bool,
    ) -> anyhow::Result<()> {
        let mut visitor = ItemVisitor {
            current_dir: file
                .as_ref()
                .parent()
                .context("Failed to get parent directory")?
                .to_path_buf(),
            v: visualizer,
            include_tests,
            pending_impls: HashMap::new(),
        };

        let content = std::fs::read_to_string(file).context("Failed to read file content")?;

        let file = syn::parse_file(&content).context("Failed to parse file")?;

        visitor.collect_impls(&file.items);

        for item in file.items {
            visitor.visit_item(&item)?;
        }

        visitor.flush_remaining_impls();

        Ok(())
    }

    fn should_skip(&self, attrs: &[syn::Attribute]) -> bool {
        if self.include_tests {
            return false;
        }

        attrs.iter().any(is_test_attribute)
    }

    fn collect_impls(&mut self, items: &[syn::Item]) {
        for item in items {
            let syn::Item::Impl(impl_items) = item else {
                continue;
            };
            if self.should_skip(&impl_items.attrs) {
                continue;
            }
            let key = type_key_from_impl(impl_items);
            self.pending_impls
                .entry(key)
                .or_default()
                .push(impl_items.clone());
        }
    }

    fn flush_impls_for(&mut self, key: &str) {
        if let Some(items) = self.pending_impls.remove(key) {
            for impl_item in items {
                self.v.push_impl(&impl_item);
            }
        }
    }

    fn flush_remaining_impls(&mut self) {
        let keys: Vec<String> = self.pending_impls.keys().cloned().collect();
        for key in keys {
            self.flush_impls_for(&key);
        }
    }

    #[expect(clippy::too_many_lines, reason = "Matches all syn::Item variants")]
    fn visit_item(&mut self, item: &syn::Item) -> anyhow::Result<()> {
        match item {
            syn::Item::Const(item_const) => {
                if self.should_skip(&item_const.attrs) {
                    return Ok(());
                }
                self.v.push_const(item_const);
            }
            syn::Item::Enum(item_enum) => {
                if self.should_skip(&item_enum.attrs) {
                    return Ok(());
                }
                self.v.push_enum(item_enum);
                let key = type_key_from_def(&item_enum.ident, &item_enum.generics);
                self.flush_impls_for(&key);
            }
            syn::Item::ExternCrate(item_extern_crate) => {
                log::warn!("Ignoring extern crate item: {item_extern_crate:?}");
            }
            syn::Item::Fn(item_fn) => {
                if self.should_skip(&item_fn.attrs) {
                    return Ok(());
                }
                self.v.push_fn(item_fn);
            }
            syn::Item::ForeignMod(item_foreign_mod) => {
                log::warn!("Ignoring foreign mod item: {item_foreign_mod:?}");
            }
            syn::Item::Impl(_) | syn::Item::Macro(_) | syn::Item::Type(_) | syn::Item::Use(_) => {}
            syn::Item::Mod(item_mod) => {
                if self.should_skip(&item_mod.attrs) {
                    return Ok(());
                }
                self.v.open_mod(item_mod);

                if let Some((_, items)) = &item_mod.content {
                    // For inline modules, update current_dir to point to where this module's
                    // file-backed submodules would be located (e.g., mod a { mod b; } looks for a/b.rs)
                    let nested_dir = self.current_dir.join(item_mod.ident.to_string());

                    let mut module_visitor = ItemVisitor {
                        current_dir: nested_dir,
                        v: self.v,
                        include_tests: self.include_tests,
                        pending_impls: HashMap::new(),
                    };
                    module_visitor.collect_impls(items);
                    for item in items {
                        module_visitor.visit_item(item)?;
                    }
                    module_visitor.flush_remaining_impls();
                } else {
                    // load the module from the file system
                    let possibilities = &[
                        self.current_dir.join(format!("{}.rs", item_mod.ident)),
                        self.current_dir.join(format!("{}/mod.rs", item_mod.ident)),
                    ];

                    let mut found = false;
                    for path in possibilities {
                        if path.exists() {
                            log::info!("Loading module from: {}", path.display());
                            ItemVisitor::visit_file(path, self.v, self.include_tests)?;
                            found = true;
                            break;
                        }
                    }

                    anyhow::ensure!(
                        found,
                        "Module {} not found in any of the expected paths: {:?}",
                        item_mod.ident,
                        possibilities
                    );
                }

                self.v.close_mod(item_mod);
            }
            syn::Item::Static(item_static) => {
                if self.should_skip(&item_static.attrs) {
                    return Ok(());
                }
                self.v.push_static(item_static);
            }
            syn::Item::Struct(item_struct) => {
                if self.should_skip(&item_struct.attrs) {
                    return Ok(());
                }
                self.v.push_struct(item_struct);
                let key = type_key_from_def(&item_struct.ident, &item_struct.generics);
                self.flush_impls_for(&key);
            }
            syn::Item::Trait(item_trait) => {
                if self.should_skip(&item_trait.attrs) {
                    return Ok(());
                }
                self.v.push_trait(item_trait);
                let key = type_key_from_def(&item_trait.ident, &item_trait.generics);
                self.flush_impls_for(&key);
            }
            syn::Item::TraitAlias(item_trait_alias) => {
                log::warn!("Ignoring trait alias item: {item_trait_alias:?}");
            }
            syn::Item::Union(item_union) => {
                log::warn!("Ignoring union item: {item_union:?}");
            }
            syn::Item::Verbatim(token_stream) => {
                log::warn!("Ignoring verbatim item: {token_stream:?}");
            }
            _ => {
                log::warn!("Ignoring unsupported item: {item:?}");
            }
        }

        Ok(())
    }
}

fn is_test_attribute(attr: &syn::Attribute) -> bool {
    let path = attr.path();
    if path.is_ident("test") {
        return true;
    }

    if path
        .segments
        .last()
        .is_some_and(|segment| segment.ident == "test")
    {
        return true;
    }

    if !path.is_ident("cfg") {
        return false;
    }

    let mut is_test = false;
    let _ = attr.parse_nested_meta(|meta| {
        if meta.path.is_ident("test") {
            is_test = true;
        }
        Ok(())
    });

    is_test
}

fn type_key_from_def(ident: &syn::Ident, generics: &syn::Generics) -> String {
    let mut text = ident.to_string();
    if !generics.params.is_empty() {
        // Only include the angle brackets and params, not the where clause
        text.push('<');
        text.push_str(&generics.params.to_token_stream().to_string());
        text.push('>');
    }
    type_key_from_tokens(&text)
}

fn type_key_from_impl(impl_items: &syn::ItemImpl) -> String {
    match impl_items.self_ty.as_ref() {
        syn::Type::Path(type_path) => {
            // Use the full path, not just the last segment
            let mut text = String::new();
            for (i, segment) in type_path.path.segments.iter().enumerate() {
                if i > 0 {
                    text.push_str("::");
                }
                text.push_str(&segment.ident.to_string());
                if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                    text.push_str(&args.to_token_stream().to_string());
                }
            }
            type_key_from_tokens(&text)
        }
        syn::Type::Array(_)
        | syn::Type::BareFn(_)
        | syn::Type::Group(_)
        | syn::Type::ImplTrait(_)
        | syn::Type::Infer(_)
        | syn::Type::Macro(_)
        | syn::Type::Never(_)
        | syn::Type::Paren(_)
        | syn::Type::Ptr(_)
        | syn::Type::Reference(_)
        | syn::Type::Slice(_)
        | syn::Type::TraitObject(_)
        | syn::Type::Tuple(_)
        | syn::Type::Verbatim(_)
        | _ => type_key_from_tokens(&impl_items.self_ty.to_token_stream().to_string()),
    }
}

fn type_key_from_tokens(raw: &str) -> String {
    raw.chars().filter(|ch| !ch.is_whitespace()).collect()
}

pub trait Visualizer {
    fn open_mod(&mut self, item_mod: &syn::ItemMod);
    fn close_mod(&mut self, item_mod: &syn::ItemMod);

    fn push_const(&mut self, item_const: &syn::ItemConst);
    fn push_enum(&mut self, item_enum: &syn::ItemEnum);
    fn push_fn(&mut self, item_fn: &syn::ItemFn);
    fn push_impl(&mut self, impl_items: &syn::ItemImpl);
    fn push_static(&mut self, item_static: &syn::ItemStatic);
    fn push_struct(&mut self, item_struct: &syn::ItemStruct);
    fn push_trait(&mut self, item_trait: &syn::ItemTrait);
}
