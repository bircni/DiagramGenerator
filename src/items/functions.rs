use anyhow::Context as _;
use itertools::Itertools as _;
use serde::Serialize;
use syn::{Visibility, spanned::Spanned as _};
use tinytemplate::TinyTemplate;

use super::ToHtml;

const FUNCTION_TEMPLATE: &str = r#"
    <div class="function">
        <div class="function-signature">
            {{ if visibility }}<span class="function-visibility">{visibility}</span> {{ endif }}
            {{ if modifiers }}<span class="function-modifiers">{modifiers}</span> {{ endif }}
            <span class="function-name">{name}</span>
            <span class="function-params">({params})</span>
            {{ if return_type }}<span class="function-return"> -> {return_type}</span>{{ endif }}
        </div>
    </div>
"#;

#[derive(Serialize)]
pub struct FunctionContext {
    pub name: String,
    pub params: String,
    pub return_type: Option<String>,
    pub visibility: Option<String>,
    pub modifiers: Option<String>,
}

impl FunctionContext {
    /// Create a `FunctionContext` from a `syn::Signature` and attributes
    pub fn new(sig: &syn::Signature, vis: &Visibility) -> Self {
        let visibility = match vis {
            Visibility::Public(_) => Some("pub".to_owned()),
            Visibility::Restricted(_) | Visibility::Inherited => None,
        };

        let mut modifiers = Vec::new();
        if sig.asyncness.is_some() {
            modifiers.push("async".to_owned());
        }
        if sig.constness.is_some() {
            modifiers.push("const".to_owned());
        }
        if sig.unsafety.is_some() {
            modifiers.push("unsafe".to_owned());
        }

        let params = sig
            .inputs
            .iter()
            .map(|param| match param {
                syn::FnArg::Receiver(r) => {
                    r.span().source_text().expect("Could not get source_text")
                }
                syn::FnArg::Typed(t) => t.span().source_text().expect("Could not get source_text"),
            })
            .join(", ");

        let return_type = match &sig.output {
            syn::ReturnType::Default => None,
            syn::ReturnType::Type(_, ty) => {
                Some(ty.span().source_text().expect("Could not get source_text"))
            }
        };

        Self {
            name: sig.ident.to_string(),
            params,
            return_type,
            visibility,
            modifiers: if modifiers.is_empty() {
                None
            } else {
                Some(modifiers.join(" "))
            },
        }
    }
}

impl ToHtml for FunctionContext {
    fn to_html(&self) -> anyhow::Result<String> {
        let mut tt = TinyTemplate::new();
        tt.set_default_formatter(&tinytemplate::format_unescaped);

        tt.add_template("function", FUNCTION_TEMPLATE)
            .context("Failed to add template")?;

        tt.render("function", self)
            .context("Failed to render template")
    }
}
