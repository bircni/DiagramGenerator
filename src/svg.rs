use anyhow::Context as _;
use quote::ToTokens;
use svg::Node;

pub struct SvgWriter {
    output: std::path::PathBuf,
    position: (f32, f32),
    svg: svg::Document,
    open_mods: Vec<(f32, f32, String)>,
    id_counter: usize,
}

const DEFAULT_TEXT_COLOR: &str = "#222";
const COLOR_PUBLIC: &str = "#111";
const COLOR_CRATE: &str = "#333";
const COLOR_RESTRICTED: &str = "#444";
const COLOR_PRIVATE: &str = "#777";

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

    fn write_line_at(&mut self, x: f32, text: impl Into<String>) {
        self.write_line_at_colored(x, text, DEFAULT_TEXT_COLOR);
    }

    fn write_line_at_colored(&mut self, x: f32, text: impl Into<String>, color: &str) {
        self.svg.append(
            svg::node::element::Text::new(text.into())
                .set("x", x)
                .set("y", self.position.1)
                .set("fill", color),
        );
        self.position.1 += 20.0;
    }

    fn write_line(&mut self, text: impl Into<String>) {
        self.write_line_at(self.position.0, text);
    }

    fn write_line_colored(&mut self, text: impl Into<String>, color: &str) {
        self.write_line_at_colored(self.position.0, text, color);
    }
}

impl crate::logic::Visualizer for SvgWriter {
    fn open_mod(&mut self, item_mod: &syn::ItemMod) {
        let id = self.next_id();

        self.open_mods
            .push((self.position.0, self.position.1, id.clone()));

        let vis = format_visibility(&item_mod.vis);
        let color = visibility_color(&item_mod.vis);
        self.svg.append(
            svg::node::element::Text::new(format!("{vis}mod {}", item_mod.ident))
                .set("x", self.position.0 + 10.0)
                .set("y", self.position.1 + 15.0)
                .set("fill", color),
        );
        self.svg
            .append(svg::node::element::Rectangle::new().set("id", id));

        self.position.1 += 30.0;
        self.position.0 += 20.0;
    }

    #[expect(clippy::expect_used, reason = "Should never fail unless there's a bug")]
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

    fn push_const(&mut self, item_const: &syn::ItemConst) {
        let vis = format_visibility(&item_const.vis);
        let color = visibility_color(&item_const.vis);
        let ty = format_type(&item_const.ty);
        self.write_line_colored(format!("{vis}const {}: {ty}", item_const.ident), color);
    }

    fn push_enum(&mut self, item_enum: &syn::ItemEnum) {
        let vis = format_visibility(&item_enum.vis);
        let color = visibility_color(&item_enum.vis);
        let generics = format_generics(&item_enum.generics);
        self.write_line_colored(format!("{vis}enum {}{generics} {{", item_enum.ident), color);

        let field_x = self.position.0 + 20.0;
        for variant in &item_enum.variants {
            match &variant.fields {
                syn::Fields::Named(fields) => {
                    let mut parts = Vec::new();
                    for field in &fields.named {
                        let name = field
                            .ident
                            .as_ref()
                            .map_or_else(|| "_".to_owned(), std::string::ToString::to_string);
                        let field_vis = format_visibility(&field.vis);
                        let ty = format_type(&field.ty);
                        parts.push(format!("{field_vis}{name}: {ty}"));
                    }
                    let fields_text = parts.join(", ");
                    self.write_line_at(field_x, format!("{} {{ {fields_text} }}", variant.ident));
                }
                syn::Fields::Unnamed(fields) => {
                    let mut parts = Vec::new();
                    for field in &fields.unnamed {
                        let field_vis = format_visibility(&field.vis);
                        parts.push(format!("{field_vis}{}", format_type(&field.ty)));
                    }
                    let fields_text = parts.join(", ");
                    self.write_line_at(field_x, format!("{}({fields_text})", variant.ident));
                }
                syn::Fields::Unit => {
                    self.write_line_at(field_x, format!("{}", variant.ident));
                }
            }
        }

        self.write_line("}".to_owned());
    }

    fn push_fn(&mut self, item_fn: &syn::ItemFn) {
        let vis = format_visibility(&item_fn.vis);
        let color = visibility_color(&item_fn.vis);
        let generics = format_generics(&item_fn.sig.generics);
        let ret = format_return_type(&item_fn.sig.output);
        self.write_line_colored(
            format!("{vis}fn {}{generics}(...){}", item_fn.sig.ident, ret),
            color,
        );
    }

    fn push_static(&mut self, item_static: &syn::ItemStatic) {
        let vis = format_visibility(&item_static.vis);
        let color = visibility_color(&item_static.vis);
        let ty = format_type(&item_static.ty);
        let mutability = if matches!(item_static.mutability, syn::StaticMutability::Mut(_)) {
            " mut"
        } else {
            ""
        };
        self.write_line_colored(
            format!("{vis}static{mutability} {}: {ty}", item_static.ident),
            color,
        );
    }

    fn push_struct(&mut self, item_struct: &syn::ItemStruct) {
        let vis = format_visibility(&item_struct.vis);
        let color = visibility_color(&item_struct.vis);
        let generics = format_generics(&item_struct.generics);
        match &item_struct.fields {
            syn::Fields::Named(fields) => {
                self.write_line_colored(
                    format!("{vis}struct {}{generics} {{", item_struct.ident),
                    color,
                );
                let field_x = self.position.0 + 20.0;
                for field in &fields.named {
                    let field_vis = format_visibility(&field.vis);
                    let field_color = visibility_color(&field.vis);
                    let name = field
                        .ident
                        .as_ref()
                        .map_or_else(|| "_".to_owned(), std::string::ToString::to_string);
                    let ty = format_type(&field.ty);
                    self.write_line_at_colored(
                        field_x,
                        format!("{field_vis}{name}: {ty}"),
                        field_color,
                    );
                }
                self.write_line("}".to_owned());
            }
            syn::Fields::Unnamed(fields) => {
                let mut parts = Vec::new();
                for field in &fields.unnamed {
                    let field_vis = format_visibility(&field.vis);
                    parts.push(format!("{field_vis}{}", format_type(&field.ty)));
                }
                let fields_text = parts.join(", ");
                self.write_line_colored(
                    format!(
                        "{vis}struct {}{generics}({fields_text});",
                        item_struct.ident
                    ),
                    color,
                );
            }
            syn::Fields::Unit => {
                self.write_line_colored(
                    format!("{vis}struct {}{generics};", item_struct.ident),
                    color,
                );
            }
        }
    }

    fn push_trait(&mut self, item_trait: &syn::ItemTrait) {
        let vis = format_visibility(&item_trait.vis);
        let color = visibility_color(&item_trait.vis);
        let generics = format_generics(&item_trait.generics);
        self.write_line_colored(
            format!("{vis}trait {}{generics} {{ ... }}", item_trait.ident),
            color,
        );
    }

    fn push_impl(&mut self, impl_items: &syn::ItemImpl) {
        let generics = format_generics(&impl_items.generics);
        let self_ty = format_type(&impl_items.self_ty);
        let header = if let Some((_, trait_path, _)) = &impl_items.trait_ {
            let trait_name = format_path(trait_path);
            format!("impl{generics} {trait_name} for {self_ty} {{")
        } else {
            format!("impl{generics} {self_ty} {{")
        };
        self.write_line(header);
        let item_x = self.position.0 + 20.0;

        for item in &impl_items.items {
            match item {
                syn::ImplItem::Const(impl_item_const) => {
                    let vis = format_visibility(&impl_item_const.vis);
                    let color = visibility_color(&impl_item_const.vis);
                    let ty = format_type(&impl_item_const.ty);
                    self.write_line_at_colored(
                        item_x,
                        format!("{vis}const {}: {ty}", impl_item_const.ident),
                        color,
                    );
                }
                syn::ImplItem::Fn(impl_item_fn) => {
                    let vis = format_visibility(&impl_item_fn.vis);
                    let color = visibility_color(&impl_item_fn.vis);
                    let generics = format_generics(&impl_item_fn.sig.generics);
                    let ret = format_return_type(&impl_item_fn.sig.output);
                    self.write_line_at_colored(
                        item_x,
                        format!("{vis}fn {}{generics}(...){}", impl_item_fn.sig.ident, ret),
                        color,
                    );
                }
                syn::ImplItem::Type(impl_item_type) => {
                    let vis = format_visibility(&impl_item_type.vis);
                    let color = visibility_color(&impl_item_type.vis);
                    let ty = format_type(&impl_item_type.ty);
                    self.write_line_at_colored(
                        item_x,
                        format!("{vis}type {} = {ty}", impl_item_type.ident),
                        color,
                    );
                }
                syn::ImplItem::Macro(_impl_item_macro) => {}
                syn::ImplItem::Verbatim(_token_stream) => {}
                _ => {
                    log::warn!("Ignoring unsupported impl item: {item:?}");
                }
            }
        }
        self.write_line("}".to_owned());
    }
}

pub fn format_visibility(vis: &syn::Visibility) -> String {
    match vis {
        syn::Visibility::Public(_) => "pub ".to_owned(),
        syn::Visibility::Restricted(restricted) => {
            let path = format_path(&restricted.path);
            if path == "crate" {
                "pub(crate) ".to_owned()
            } else {
                format!("pub(in {path}) ")
            }
        }
        syn::Visibility::Inherited => String::new(),
    }
}

fn visibility_color(vis: &syn::Visibility) -> &'static str {
    match vis {
        syn::Visibility::Public(_) => COLOR_PUBLIC,
        syn::Visibility::Restricted(restricted) => {
            let path = format_path(&restricted.path);
            if path == "crate" {
                COLOR_CRATE
            } else {
                COLOR_RESTRICTED
            }
        }
        syn::Visibility::Inherited => COLOR_PRIVATE,
    }
}

pub fn format_generics(generics: &syn::Generics) -> String {
    if generics.params.is_empty() {
        return String::new();
    }

    let mut parts = Vec::new();
    for param in &generics.params {
        match param {
            syn::GenericParam::Type(ty) => parts.push(ty.ident.to_string()),
            syn::GenericParam::Lifetime(lt) => parts.push(lt.lifetime.to_string()),
            syn::GenericParam::Const(cnst) => parts.push(format!("const {}", cnst.ident)),
        }
    }

    format!("<{}>", parts.join(", "))
}

pub fn format_return_type(output: &syn::ReturnType) -> String {
    match output {
        syn::ReturnType::Default => String::new(),
        syn::ReturnType::Type(_, ty) => format!(" -> {}", format_type(ty)),
    }
}

pub fn format_type<T: ToTokens>(ty: &T) -> String {
    compact_tokens(&ty.to_token_stream().to_string())
}

pub fn format_path(path: &syn::Path) -> String {
    compact_tokens(&path.to_token_stream().to_string())
}

fn compact_tokens(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    let mut chars = raw.chars().peekable();
    let mut prev: Option<char> = None;

    while let Some(ch) = chars.next() {
        if ch.is_whitespace() {
            while let Some(next) = chars.peek() {
                if next.is_whitespace() {
                    chars.next();
                } else {
                    break;
                }
            }

            let Some(next) = chars.peek().copied() else {
                break;
            };

            if prev.is_some_and(is_tight_punct) || is_tight_punct(next) {
                continue;
            }

            if prev == Some(',') {
                if !matches!(next, ')' | '>' | ']' | '}') {
                    out.push(' ');
                    prev = Some(' ');
                }
                continue;
            }

            out.push(' ');
            prev = Some(' ');
            continue;
        }

        if ch == ',' {
            if out.ends_with(' ') {
                out.pop();
            }
            out.push(',');
            prev = Some(',');
            continue;
        }

        out.push(ch);
        prev = Some(ch);
    }

    out
}

const fn is_tight_punct(ch: char) -> bool {
    matches!(
        ch,
        ':' | '<' | '>' | '(' | ')' | '[' | ']' | '{' | '}' | '&' | '*' | '='
    )
}
