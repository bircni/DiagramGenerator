use crate::logic::{organize_and_render_items, parse_file_recursive};
use std::path::PathBuf;

#[test]
fn test_parse_file_recursive_valid_path() {
    let path = PathBuf::from("src/main.rs");
    let result = parse_file_recursive(&path, false);
    result.unwrap();
}

#[test]
fn test_parse_file_recursive_invalid_path() {
    let path = PathBuf::from("invalid/path.rs");
    let result = parse_file_recursive(&path, false);
    result.unwrap_err();
}

#[test]
fn test_organize_and_render_items_empty() {
    let path = PathBuf::from("src/main.rs");
    let items = vec![];
    let result = organize_and_render_items(&path, items, false, false);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "");
}
