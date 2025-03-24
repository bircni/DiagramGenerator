pub mod module;
pub mod structs;

pub trait ToHtml {
    fn to_html(&self) -> String;
}
