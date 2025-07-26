use anyhow::Context;
use serde::Serialize;
use tinytemplate::TinyTemplate;

use super::ToHtml;

const MODULE_TEMPLATE: &str = r#"
    <div class="module">
        <input type="checkbox" id="module-{name}" class="module-toggle" checked>
        <label for="module-{name}" class="module-header">
            <span class="toggle-icon">▼</span>
            <span class="module-name">{name}</span>
        </label>
        <div class="module-contents">
{contents}
        </div>
    </div>
"#;

#[derive(Serialize)]
pub struct ModContext {
    pub name: String,
    pub contents: String,
}

impl ToHtml for ModContext {
    fn to_html(&self) -> anyhow::Result<String> {
        let mut tt = TinyTemplate::new();
        tt.set_default_formatter(&tinytemplate::format_unescaped);

        tt.add_template("module", MODULE_TEMPLATE)
            .context("Failed to add template")?;

        tt.render("module", self)
            .context("Failed to render template")
    }
}
