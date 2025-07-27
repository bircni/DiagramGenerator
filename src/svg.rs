use anyhow::Context as _;
use svg::Node;
use syn::spanned::Spanned as _;

pub struct SvgWriter {
    output: std::path::PathBuf,
    position: (f32, f32),
    svg: svg::Document,
    open_mods: Vec<(f32, f32, String)>,
    id_counter: usize,
}

impl SvgWriter {
    pub fn new(output: std::path::PathBuf) -> Self {
        let svg = svg::Document::new()
            .set("xmlns", "http://www.w3.org/2000/svg")
            .set("font-family", "monospace")
            .set("font-size", "12")
            .set("style", "background-color: white");

        Self {
            output,
            svg,
            position: (20.0, 20.0),
            open_mods: vec![],
            id_counter: 0,
        }
    }

    pub fn finish(mut self) -> anyhow::Result<()> {
        self.svg = self
            .svg
            .set("width", format!("{}px", self.position.0 + 1000.0 + 40.0))
            .set("height", format!("{}px", self.position.1 + 40.0));

        let mut file =
            std::fs::File::create(&self.output).context("Failed to create output SVG file")?;
        svg::write(&mut file, &self.svg).context("Failed to write SVG content to file")?;
        Ok(())
    }

    fn next_id(&mut self) -> String {
        let id = self.id_counter;
        self.id_counter += 1;
        format!("{id}")
    }
}

impl crate::logic::Visualizer for SvgWriter {
    fn open_mod(&mut self, item_mod: &syn::ItemMod) {
        let id = self.next_id();

        self.open_mods
            .push((self.position.0, self.position.1, id.clone()));

        self.svg.append(
            svg::node::element::Text::new(format!("mod {}", item_mod.ident))
                .set("x", self.position.0 + 10.0)
                .set("y", self.position.1 - 5.0),
        );
        self.svg
            .append(svg::node::element::Rectangle::new().set("id", id));

        self.position.1 += 20.0;
        self.position.0 += 20.0;
    }

    fn close_mod(&mut self, _: &syn::ItemMod) {
        let (x, y, id) = self.open_mods.pop().expect("No open module to close");

        let mut binding = self.svg.get_children_mut();
        let attrs = binding
            .iter_mut()
            .flat_map(|cs| cs.iter_mut())
            .find(|n| {
                n.get_attributes().is_some_and(|attrs| {
                    attrs.get("id") == Some(&svg::node::Value::from(id.clone()))
                })
            })
            .expect("Failed to find module rectangle")
            .get_attributes_mut()
            .expect("Failed to get attributes");

        attrs.insert("x".into(), x.into());
        attrs.insert("y".into(), y.into());
        attrs.insert("width".into(), 1000.0.into());
        attrs.insert("height".into(), (self.position.1 - y + 20.0).into());
        attrs.insert("fill".into(), "rgba(250, 255, 204, 0.5)".into());
        attrs.insert("stroke".into(), "black".into());
        attrs.insert("rx".into(), 5.into());
        attrs.insert("ry".into(), 5.into());

        self.position.0 -= 20.0;
        self.position.1 += 60.0;
    }

    fn push_const(&mut self, item_const: &syn::ItemConst) {}

    fn push_enum(&mut self, item_enum: &syn::ItemEnum) {}

    fn push_fn(&mut self, item_fn: &syn::ItemFn) {
        self.svg.append(
            svg::node::element::Text::new(format!(
                "fn {}(...) {}",
                item_fn.sig.ident,
                item_fn.sig.output.span().source_text().unwrap_or_default()
            ))
            .set("x", self.position.0)
            .set("y", self.position.1),
        );

        self.position.1 += 20.0;
    }

    fn push_static(&mut self, item_static: &syn::ItemStatic) {}

    fn push_struct(&mut self, item_struct: &syn::ItemStruct) {
        self.svg.append(
            svg::node::element::Text::new(format!("struct {} {{...}}", item_struct.ident))
                .set("x", self.position.0)
                .set("y", self.position.1),
        );

        self.position.1 += 20.0;
    }

    fn push_trait(&mut self, item_trait: &syn::ItemTrait) {
        self.svg.append(
            svg::node::element::Text::new(format!("trait {} {{ ... }}", item_trait.ident))
                .set("x", self.position.0)
                .set("y", self.position.1),
        );

        self.position.1 += 20.0;
    }

    fn push_impl(&mut self, impl_items: &syn::ItemImpl) {
        self.svg.append(
            svg::node::element::Text::new(format!(
                "impl {} {{",
                impl_items.self_ty.span().source_text().unwrap_or_default()
            ))
            .set("x", self.position.0)
            .set("y", self.position.1),
        );
        self.position.1 += 20.0;
        self.position.0 += 20.0;

        for item in &impl_items.items {
            match item {
                syn::ImplItem::Const(impl_item_const) => {}
                syn::ImplItem::Fn(impl_item_fn) => {
                    self.svg.append(
                        svg::node::element::Text::new(format!(
                            "fn {}(...) {}",
                            impl_item_fn.sig.ident,
                            impl_item_fn
                                .sig
                                .output
                                .span()
                                .source_text()
                                .unwrap_or_default()
                        ))
                        .set("x", self.position.0)
                        .set("y", self.position.1),
                    );
                    self.position.1 += 20.0;
                }
                syn::ImplItem::Type(impl_item_type) => {}
                syn::ImplItem::Macro(impl_item_macro) => {}
                syn::ImplItem::Verbatim(token_stream) => {}
                _ => {
                    log::warn!("Ignoring unsupported impl item: {item:?}");
                }
            }
        }

        self.position.0 -= 20.0;

        self.svg.append(
            svg::node::element::Text::new("}")
                .set("x", self.position.0)
                .set("y", self.position.1),
        );
        self.position.1 += 20.0;
    }
}

mod test {
    struct Teeeest;
}
