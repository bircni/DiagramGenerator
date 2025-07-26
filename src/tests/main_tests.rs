use crate::find_path;

use std::path::PathBuf;

#[test]
fn test_find_path_with_file() {
    let file_path = PathBuf::from("src/main.rs");
    let result = find_path(Some(file_path.clone()));
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), file_path);
}

#[test]
fn test_find_path_with_directory() {
    let dir_path = PathBuf::from("src");
    let result = find_path(Some(dir_path));
    assert!(result.is_ok());
    let found_path = result.unwrap();
    assert!(found_path.ends_with("main.rs") || found_path.ends_with("lib.rs"));
}

#[test]
fn test_find_path_without_input() {
    let result = find_path(None);
    assert!(result.is_ok());
    let found_path = result.unwrap();
    assert!(found_path.ends_with("main.rs") || found_path.ends_with("lib.rs"));
}
