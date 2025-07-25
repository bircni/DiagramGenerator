pub mod enums;
pub mod functions;
pub mod impl_blocks;
pub mod module;
pub mod structs;

pub trait ToHtml {
    fn to_html(&self) -> anyhow::Result<String>;
}
