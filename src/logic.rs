use anyhow::Context as _;
use syn::spanned::Spanned;

pub struct ItemVisitor<'a, V> {
    current_dir: std::path::PathBuf,
    v: &'a mut V,
}

impl<'a, V: Visualizer> ItemVisitor<'a, V> {
    pub fn visit_file(
        file: impl AsRef<std::path::Path>,
        visualizer: &'a mut V,
    ) -> anyhow::Result<()> {
        let mut visitor = ItemVisitor {
            current_dir: file
                .as_ref()
                .parent()
                .context("Failed to get parent directory")?
                .to_path_buf(),
            v: visualizer,
        };

        let content = std::fs::read_to_string(file).context("Failed to read file content")?;

        let file = syn::parse_file(&content).context("Failed to parse file")?;

        for item in file.items {
            visitor.visit_item(&item)?;
        }

        Ok(())
    }

    fn visit_item(&mut self, item: &syn::Item) -> anyhow::Result<()> {
        match item {
            syn::Item::Const(item_const) => self.v.push_const(item_const),
            syn::Item::Enum(item_enum) => self.v.push_enum(item_enum),
            syn::Item::ExternCrate(_) => {
                unimplemented!("external crate not supported yet")
            }
            syn::Item::Fn(item_fn) => self.v.push_fn(item_fn),
            syn::Item::ForeignMod(_) => {
                unimplemented!("foreign mod not supported yet")
            }
            syn::Item::Impl(impl_items) => self.v.push_impl(impl_items),
            syn::Item::Macro(_) => {}
            syn::Item::Mod(item_mod) => {
                self.v.open_mod(item_mod);

                if let Some((_, items)) = &item_mod.content {
                    for item in items {
                        self.visit_item(item)?;
                    }
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
                            ItemVisitor::visit_file(path, self.v)?;
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
            syn::Item::Static(item_static) => self.v.push_static(item_static),
            syn::Item::Struct(item_struct) => self.v.push_struct(item_struct),
            syn::Item::Trait(item_trait) => self.v.push_trait(item_trait),
            syn::Item::TraitAlias(_) => {
                unimplemented!("trait alias not supported yet")
            }
            syn::Item::Type(_) => {}
            syn::Item::Union(_) => unimplemented!("union not supported yet"),
            syn::Item::Use(_) => {}
            syn::Item::Verbatim(token_stream) => {
                log::warn!("Ignoring verbatim item: {:?}", token_stream);
            }
            _ => {
                log::warn!("Ignoring unsupported item: {:?}", item);
            }
        }

        Ok(())
    }
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
