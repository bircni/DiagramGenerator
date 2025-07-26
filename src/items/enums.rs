use anyhow::Context as _;
use serde::Serialize;
use tinytemplate::TinyTemplate;

use super::ToHtml;

const ENUM_TEMPLATE: &str = r#"
    <div class="enum">
        <div class="enum-name">{name}</div>
        <div class="enum-variants">
            {{ for variant in variants }}
                <div class="enum-variant">
                    <div class="enum-variant-name">{variant.name}</div>
                    {{ if variant.data }}
                        <div class="enum-variant-data">{variant.data}</div>
                    {{ endif }}
                </div>
            {{ endfor }}
        </div>
    </div>
"#;

#[derive(Serialize)]
pub struct EnumContext {
    pub name: String,
    pub variants: Vec<EnumVariantContext>,
}

#[derive(Serialize)]
pub struct EnumVariantContext {
    pub name: String,
    pub data: Option<String>,
}

impl ToHtml for EnumContext {
    fn to_html(&self) -> anyhow::Result<String> {
        let mut tt = TinyTemplate::new();
        tt.set_default_formatter(&tinytemplate::format_unescaped);

        tt.add_template("enum", ENUM_TEMPLATE)
            .context("Failed to add template")?;

        tt.render("enum", self).context("Failed to render template")
    }
}
