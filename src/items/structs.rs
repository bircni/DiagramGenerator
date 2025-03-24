use serde::Serialize;
use tinytemplate::TinyTemplate;

const STRUCT_TEMPLATE: &str = r#"
    <div class="struct">
        <div class="struct-name">{name}</div>
        <div class="struct-public-fields">
            {{ for field in public_fields }}
                <div class="struct-field">
                    <div class="struct-field-name">{field.name}</div>
                    <div class="struct-field-type">{field.type_}</div>
                </div>
            {{ endfor }}
        </div>
        <div class="struct-private-fields">
            {{ for field in private_fields }}
                <div class="struct-field">
                    <div class="struct-field-name">{field.name}</div>
                    <div class="struct-field-type">{field.type_}</div>
                </div>
            {{ endfor }}
        </div>
    </div>
"#;

#[derive(Serialize)]
pub struct StructContext {
    pub name: String,
    pub public_fields: Vec<StructFieldContext>,
    pub private_fields: Vec<StructFieldContext>,
}

#[derive(Serialize)]
pub struct StructFieldContext {
    pub name: String,
    pub type_: String,
}

impl StructContext {
    pub fn to_html(&self) -> String {
        let mut tt = TinyTemplate::new();

        tt.add_template("struct", STRUCT_TEMPLATE)
            .expect("Failed to add template");

        tt.render("struct", self)
            .expect("Failed to render template")
    }
}
