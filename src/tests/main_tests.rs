use crate::format_output_filename;
use cargo::core::manifest::TargetKind;

#[test]
fn test_format_output_filename_bin() {
    let kind = TargetKind::Bin;
    assert_eq!(
        format_output_filename("mypackage", &kind),
        "mypackage-bin.svg"
    );
}

#[test]
fn test_format_output_filename_test() {
    let kind = TargetKind::Test;
    assert_eq!(
        format_output_filename("mypackage", &kind),
        "mypackage-test.svg"
    );
}

#[test]
fn test_format_output_filename_bench() {
    let kind = TargetKind::Bench;
    assert_eq!(
        format_output_filename("mypackage", &kind),
        "mypackage-bench.svg"
    );
}

#[test]
fn test_format_output_filename_example_bin() {
    let kind = TargetKind::ExampleBin;
    assert_eq!(
        format_output_filename("example1", &kind),
        "example1-example-bin.svg"
    );
}

#[test]
fn test_format_output_filename_build() {
    let kind = TargetKind::CustomBuild;
    assert_eq!(format_output_filename("build", &kind), "build-build.svg");
}

#[test]
fn test_format_output_filename_different_names_same_kind() {
    let kind = TargetKind::Bin;
    assert_ne!(
        format_output_filename("package1", &kind),
        format_output_filename("package2", &kind)
    );
}

/// This test documents the core issue from the review:
/// When a package has both lib.rs and main.rs with the same name,
/// they must generate different output files
#[test]
fn test_format_output_filename_prevents_collision() {
    // Simulating a package with both lib and bin targets named "mypackage"
    let lib_kind = TargetKind::Lib(vec![]);
    let bin_kind = TargetKind::Bin;

    let lib_filename = format_output_filename("mypackage", &lib_kind);
    let bin_filename = format_output_filename("mypackage", &bin_kind);

    // These MUST be different to prevent file overwriting
    assert_ne!(
        lib_filename, bin_filename,
        "Library and binary targets with same name must produce different output files"
    );

    assert_eq!(lib_filename, "mypackage-lib.svg");
    assert_eq!(bin_filename, "mypackage-bin.svg");
}

/// Comprehensive test that simulates a realistic Cargo package with multiple targets
/// This directly addresses the review comment scenario
#[test]
fn test_multiple_targets_same_name_no_collision() {
    let target_name = "common_name";

    // Common scenario: lib + bin with same name (package name)
    let lib = format_output_filename(target_name, &TargetKind::Lib(vec![]));
    let bin = format_output_filename(target_name, &TargetKind::Bin);
    let test = format_output_filename(target_name, &TargetKind::Test);
    let bench = format_output_filename(target_name, &TargetKind::Bench);

    // Verify all are unique
    let all_filenames = [&lib, &bin, &test, &bench];
    for (i, filename1) in all_filenames.iter().enumerate() {
        for (j, filename2) in all_filenames.iter().enumerate() {
            if i != j {
                assert_ne!(
                    filename1, filename2,
                    "Filenames must be unique: {filename1} vs {filename2}"
                );
            }
        }
    }

    // Verify expected format
    assert_eq!(lib, "common_name-lib.svg");
    assert_eq!(bin, "common_name-bin.svg");
    assert_eq!(test, "common_name-test.svg");
    assert_eq!(bench, "common_name-bench.svg");
}
