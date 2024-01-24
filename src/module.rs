use serde::Serialize;
use tinytemplate::TinyTemplate;

use crate::ToHtml;

const MODULE_TEMPLATE: &str = r#"
    <div class="module">
        <div class="module-name">{name}</div>
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
    fn to_html(&self) -> String {
        let mut tt = TinyTemplate::new();
        tt.set_default_formatter(&tinytemplate::format_unescaped);

        tt.add_template("module", MODULE_TEMPLATE)
            .expect("Failed to add template");

        tt.render("module", self)
            .expect("Failed to render template")
    }
}
