use crate::svg::{format_generics, format_path, format_type, format_visibility};

#[test]
fn formats_visibility() {
    let item: syn::ItemStruct = syn::parse_str("pub(crate) struct Example;").unwrap();
    assert_eq!(format_visibility(&item.vis), "pub(crate) ");
}

#[test]
fn formats_generics_and_types() {
    let item: syn::ItemStruct =
        syn::parse_str("pub struct Example<'a, T, const N: usize> { value: &'a T }").unwrap();
    assert_eq!(format_generics(&item.generics), "<'a, T, const N>");
    let field = item.fields.iter().next().unwrap();
    assert_eq!(format_type(&field.ty), "&'a T");
}

#[test]
fn formats_trait_impl_header() {
    let item: syn::ItemImpl = syn::parse_str("impl<T> core::fmt::Debug for Wrapper<T> {}").unwrap();
    let generics = format_generics(&item.generics);
    let self_ty = format_type(&item.self_ty);
    let (_, trait_path, _) = item.trait_.as_ref().unwrap();
    let header = format!(
        "impl{generics} {} for {self_ty} {{",
        format_path(trait_path)
    );
    assert_eq!(header, "impl<T> core::fmt::Debug for Wrapper<T> {");
}
