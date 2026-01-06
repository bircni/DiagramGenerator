use crate::logic::{ItemVisitor, Visualizer};
use quote::ToTokens as _;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Default)]
struct RecordingVisualizer {
    events: Vec<String>,
}

impl RecordingVisualizer {
    fn record(&mut self, label: &str, name: impl std::fmt::Display) {
        self.events.push(format!("{label}:{name}"));
    }
}

impl Visualizer for RecordingVisualizer {
    fn open_mod(&mut self, item_mod: &syn::ItemMod) {
        self.record("open_mod", &item_mod.ident);
    }

    fn close_mod(&mut self, item_mod: &syn::ItemMod) {
        self.record("close_mod", &item_mod.ident);
    }

    fn push_const(&mut self, item_const: &syn::ItemConst) {
        self.record("const", &item_const.ident);
    }

    fn push_enum(&mut self, item_enum: &syn::ItemEnum) {
        self.record("enum", &item_enum.ident);
    }

    fn push_fn(&mut self, item_fn: &syn::ItemFn) {
        self.record("fn", &item_fn.sig.ident);
    }

    fn push_impl(&mut self, impl_items: &syn::ItemImpl) {
        self.record("impl", impl_items.self_ty.to_token_stream().to_string());
    }

    fn push_static(&mut self, item_static: &syn::ItemStatic) {
        self.record("static", &item_static.ident);
    }

    fn push_struct(&mut self, item_struct: &syn::ItemStruct) {
        self.record("struct", &item_struct.ident);
    }

    fn push_trait(&mut self, item_trait: &syn::ItemTrait) {
        self.record("trait", &item_trait.ident);
    }
}

fn create_temp_dir(label: &str) -> PathBuf {
    let mut dir = std::env::temp_dir();
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    dir.push(format!("diagen-{label}-{nanos}"));
    fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn visit_file_skips_test_items_by_default() {
    let dir = create_temp_dir("skip-tests");
    let main_rs = dir.join("main.rs");
    fs::write(
        &main_rs,
        r"
fn main() {}

#[test]
fn test_one() {}

#[cfg(test)]
mod tests {
    #[test]
    fn nested() {}

    fn helper() {}
}
",
    )
    .unwrap();

    let mut visualizer = RecordingVisualizer::default();
    ItemVisitor::visit_file(&main_rs, &mut visualizer, false).unwrap();

    assert_eq!(visualizer.events, vec!["fn:main"]);
}

#[test]
fn visit_file_includes_test_items_when_enabled() {
    let dir = create_temp_dir("include-tests");
    let main_rs = dir.join("main.rs");
    fs::write(
        &main_rs,
        r"
fn main() {}

#[test]
fn test_one() {}

#[cfg(test)]
mod tests {
    #[test]
    fn nested() {}
}
",
    )
    .unwrap();

    let mut visualizer = RecordingVisualizer::default();
    ItemVisitor::visit_file(&main_rs, &mut visualizer, true).unwrap();

    assert!(visualizer.events.contains(&"fn:main".to_owned()));
    assert!(visualizer.events.contains(&"fn:test_one".to_owned()));
    assert!(visualizer.events.contains(&"open_mod:tests".to_owned()));
    assert!(visualizer.events.contains(&"fn:nested".to_owned()));
    assert!(visualizer.events.contains(&"close_mod:tests".to_owned()));
}

#[test]
fn visit_file_loads_module_from_filesystem() {
    let dir = create_temp_dir("module-file");
    let main_rs = dir.join("main.rs");
    let nested_rs = dir.join("nested.rs");
    fs::write(
        &main_rs,
        r"
mod nested;
fn main() {}
",
    )
    .unwrap();
    fs::write(&nested_rs, "struct Widget;").unwrap();

    let mut visualizer = RecordingVisualizer::default();
    ItemVisitor::visit_file(&main_rs, &mut visualizer, false).unwrap();

    assert_eq!(
        visualizer.events,
        vec![
            "open_mod:nested",
            "struct:Widget",
            "close_mod:nested",
            "fn:main",
        ]
    );
}

#[test]
fn visit_file_groups_impls_under_type() {
    let dir = create_temp_dir("group-impls");
    let main_rs = dir.join("main.rs");
    fs::write(
        &main_rs,
        r"
impl Widget {
    fn new() {}
}

pub struct Widget;
",
    )
    .unwrap();

    let mut visualizer = RecordingVisualizer::default();
    ItemVisitor::visit_file(&main_rs, &mut visualizer, false).unwrap();

    assert_eq!(visualizer.events, vec!["struct:Widget", "impl:Widget"]);
}

/// This test demonstrates the issue where impl blocks with the same type name
/// from different modules incorrectly share the same key
#[test]
fn visit_file_handles_fully_qualified_impl_paths() {
    let dir = create_temp_dir("qualified-impls");
    let main_rs = dir.join("main.rs");
    fs::write(
        &main_rs,
        r"
pub struct Widget;

impl Widget {
    fn local_method() {}
}

impl std::string::String {
    fn custom_method() {}
}

impl core::option::Option<i32> {
    fn specialized() {}
}
",
    )
    .unwrap();

    let mut visualizer = RecordingVisualizer::default();
    ItemVisitor::visit_file(&main_rs, &mut visualizer, false).unwrap();

    // Widget impl should be attached to Widget struct
    // The external type impls should appear after (not attached to any struct)
    assert!(visualizer.events.contains(&"struct:Widget".to_owned()));
    assert!(visualizer.events.contains(&"impl:Widget".to_owned()));

    // Check that struct comes before its impl
    let struct_pos = visualizer
        .events
        .iter()
        .position(|e| e == "struct:Widget")
        .unwrap();
    let impl_pos = visualizer
        .events
        .iter()
        .position(|e| e == "impl:Widget")
        .unwrap();
    assert!(struct_pos < impl_pos, "Struct should come before its impl");
    assert_eq!(
        struct_pos + 1,
        impl_pos,
        "Impl should be immediately after struct"
    );
}

/// Test that demonstrates the collision problem with same type names in different modules
#[test]
#[expect(
    clippy::similar_names,
    reason = "Test needs modules a and b for demonstration"
)]
fn visit_file_prevents_impl_collision_across_modules() {
    let dir = create_temp_dir("impl-collision");
    let main_rs = dir.join("main.rs");
    let mod_a_rs = dir.join("a.rs");
    let mod_b_rs = dir.join("b.rs");

    fs::write(
        &main_rs,
        r"
mod a;
mod b;

// This impl should NOT be grouped with a::Widget or b::Widget
impl Widget {
    fn root_method() {}
}

struct Widget;
",
    )
    .unwrap();

    fs::write(
        &mod_a_rs,
        r"
pub struct Widget;

impl Widget {
    fn a_method() {}
}
",
    )
    .unwrap();

    fs::write(
        &mod_b_rs,
        r"
pub struct Widget;

impl Widget {
    fn b_method() {}
}
",
    )
    .unwrap();

    let mut visualizer = RecordingVisualizer::default();
    ItemVisitor::visit_file(&main_rs, &mut visualizer, false).unwrap();

    // Each module's Widget should have its own impl attached
    // and not interfere with each other
    let events_str = visualizer.events.join(", ");

    // The root Widget struct and its impl should be together
    assert!(events_str.contains("struct:Widget"));
    assert!(events_str.contains("impl:Widget"));

    // Module a should have its own Widget struct and impl
    let a_start = visualizer
        .events
        .iter()
        .position(|e| e == "open_mod:a")
        .unwrap();
    let a_end = visualizer
        .events
        .iter()
        .position(|e| e == "close_mod:a")
        .unwrap();
    let a_events: Vec<_> = visualizer.events[a_start + 1..a_end].to_vec();
    assert_eq!(a_events, vec!["struct:Widget", "impl:Widget"]);

    // Module b should have its own Widget struct and impl
    let b_start = visualizer
        .events
        .iter()
        .position(|e| e == "open_mod:b")
        .unwrap();
    let b_end = visualizer
        .events
        .iter()
        .position(|e| e == "close_mod:b")
        .unwrap();
    let b_events: Vec<_> = visualizer.events[b_start + 1..b_end].to_vec();
    assert_eq!(b_events, vec!["struct:Widget", "impl:Widget"]);
}

/// This test demonstrates the actual bug: when you have impl blocks for
/// fully-qualified paths like `impl a::Widget` in the same file, they
/// collide with local Widget definitions
#[test]
#[expect(
    clippy::similar_names,
    reason = "Test needs modules a and b for demonstration"
)]
fn visit_file_distinguishes_qualified_impl_paths() {
    let dir = create_temp_dir("qualified-path-collision");
    let main_rs = dir.join("main.rs");
    let mod_a_rs = dir.join("a.rs");
    let mod_b_rs = dir.join("b.rs");

    fs::write(
        &mod_a_rs,
        r"
pub struct Widget {
    pub value: i32
}
",
    )
    .unwrap();

    fs::write(
        &mod_b_rs,
        r"
pub struct Widget {
    pub name: String
}
",
    )
    .unwrap();

    // The main file has impl blocks for a::Widget and b::Widget
    // These should NOT collide even though they both end with "Widget"
    fs::write(
        &main_rs,
        r"
mod a;
mod b;

// Impl for a::Widget - should NOT be grouped with b::Widget
impl a::Widget {
    fn from_a() {}
}

// Impl for b::Widget - different type!
impl b::Widget {
    fn from_b() {}
}

// Local Widget - yet another different type!
struct Widget;

impl Widget {
    fn local() {}
}
",
    )
    .unwrap();

    let mut visualizer = RecordingVisualizer::default();
    ItemVisitor::visit_file(&main_rs, &mut visualizer, false).unwrap();

    // Find the root-level Widget struct (not the ones in modules a or b)
    // It should be after close_mod:b
    let close_b_pos = visualizer
        .events
        .iter()
        .position(|e| e == "close_mod:b")
        .unwrap();
    let root_struct_pos = visualizer.events[close_b_pos..]
        .iter()
        .position(|e| e == "struct:Widget")
        .map(|p| close_b_pos + p)
        .unwrap();

    // The root Widget impl should be immediately after the struct
    assert_eq!(
        visualizer.events[root_struct_pos + 1],
        "impl:Widget",
        "Root Widget impl should immediately follow root struct"
    );

    // The a::Widget and b::Widget impls should appear somewhere, but not grouped with local Widget
    let has_a_widget_impl = visualizer
        .events
        .iter()
        .any(|e| e.contains('a') && e.contains("Widget"));
    let has_b_widget_impl = visualizer
        .events
        .iter()
        .any(|e| e.contains('b') && e.contains("Widget"));

    assert!(has_a_widget_impl, "Should have impl for a::Widget");
    assert!(has_b_widget_impl, "Should have impl for b::Widget");

    // Count all Widget impls - with the bug, they might all collapse into one
    let widget_impl_count = visualizer
        .events
        .iter()
        .filter(|e| e.starts_with("impl:") && e.contains("Widget"))
        .count();
    assert!(
        widget_impl_count >= 3,
        "Should have at least 3 separate Widget impl entries (local, a::Widget, b::Widget), found {widget_impl_count}"
    );
}

/// This test demonstrates the bug where where clauses in type definitions
/// cause the impl block keys to not match the type definition keys
#[test]
fn visit_file_groups_impls_with_where_clauses() {
    let dir = create_temp_dir("where-clause");
    let main_rs = dir.join("main.rs");
    fs::write(
        &main_rs,
        r"
// Impl appears BEFORE the struct definition
impl<T> Container<T>
where
    T: Copy,
{
    fn new(value: T) -> Self {
        Container { value }
    }
}

// Type definition with where clause
pub struct Container<T>
where
    T: Copy,
{
    value: T,
}

// Another impl after the definition
impl<T> Container<T>
where
    T: Copy + Clone,
{
    fn clone_value(&self) -> T {
        self.value.clone()
    }
}
",
    )
    .unwrap();

    let mut visualizer = RecordingVisualizer::default();
    ItemVisitor::visit_file(&main_rs, &mut visualizer, false).unwrap();

    // The key issue: both impls should be grouped with the struct
    // Expected: struct:Container, impl:Container<T>, impl:Container<T>
    // This test verifies that where clauses don't cause key mismatches

    // Find struct position
    let struct_pos = visualizer
        .events
        .iter()
        .position(|e| e == "struct:Container")
        .unwrap();

    // Count how many impls come right after the struct
    let mut impl_count_after_struct = 0;
    for event in &visualizer.events[struct_pos + 1..] {
        if event.starts_with("impl:") && event.contains("Container") {
            impl_count_after_struct += 1;
        } else if event.starts_with("struct:")
            || event.starts_with("enum:")
            || event.starts_with("trait:")
        {
            // Stop at next type definition
            break;
        }
    }

    assert_eq!(
        impl_count_after_struct, 2,
        "Both impl blocks should be grouped immediately after the struct, found {impl_count_after_struct}"
    );
}

/// This test demonstrates the bug where nested modules in inline mods
/// are resolved from the wrong directory
#[test]
fn visit_file_loads_nested_module_from_correct_directory() {
    let dir = create_temp_dir("nested-inline-mod");
    let main_rs = dir.join("main.rs");

    // Create directory structure: a/b.rs
    let a_dir = dir.join("a");
    fs::create_dir_all(&a_dir).unwrap();
    let b_rs = a_dir.join("b.rs");

    // The main file has an inline module 'a' which contains a file-backed module 'b'
    fs::write(
        &main_rs,
        r"
// Inline module 'a' with content
mod a {
    // File-backed module 'b' should load from a/b.rs
    mod b;
    
    pub struct InlineStruct;
}

fn main() {}
",
    )
    .unwrap();

    // Create the a/b.rs file
    fs::write(
        &b_rs,
        r"
pub struct NestedStruct;
",
    )
    .unwrap();

    let mut visualizer = RecordingVisualizer::default();

    // This should succeed and load a/b.rs correctly
    ItemVisitor::visit_file(&main_rs, &mut visualizer, false).unwrap();

    // Verify that we found the nested module
    assert!(visualizer.events.contains(&"open_mod:a".to_owned()));
    assert!(
        visualizer
            .events
            .contains(&"struct:InlineStruct".to_owned())
    );
    assert!(visualizer.events.contains(&"open_mod:b".to_owned()));
    assert!(
        visualizer
            .events
            .contains(&"struct:NestedStruct".to_owned())
    );
    assert!(visualizer.events.contains(&"close_mod:b".to_owned()));
    assert!(visualizer.events.contains(&"close_mod:a".to_owned()));
}
