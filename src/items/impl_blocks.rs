use anyhow::Context;
use serde::Serialize;
use tinytemplate::TinyTemplate;

use super::ToHtml;

const IMPL_TEMPLATE: &str = r#"
    <div class="impl-block">
        <div class="impl-header">
            <span class="impl-type">impl</span>
            {{ if generics }}<span class="impl-generics">{generics}</span>{{ endif }}
            {{ if trait_name }}<span class="impl-trait">{trait_name}</span>{{ endif }}
            <span class="impl-target"> for {target_type}</span>
        </div>
        <div class="impl-content">
            {{ for function in functions }}
                {function}
            {{ endfor }}
        </div>
    </div>
"#;

#[derive(Serialize)]
pub struct ImplContext {
    pub target_type: String,
    pub trait_name: Option<String>,
    pub generics: Option<String>,
    pub functions: Vec<String>,
}

impl ToHtml for ImplContext {
    fn to_html(&self) -> anyhow::Result<String> {
        let mut tt = TinyTemplate::new();
        tt.set_default_formatter(&tinytemplate::format_unescaped);

        tt.add_template("impl", IMPL_TEMPLATE)
            .context("Failed to add template")?;

        tt.render("impl", self).context("Failed to render template")
    }
}
