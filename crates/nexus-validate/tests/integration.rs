use std::path::Path;

/// Load the sample network.toml and confirm it passes all validation rules.
#[test]
fn sample_network_is_valid() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let sample_path = Path::new(manifest_dir).join("../../examples/sample/network.toml");

    let network = nexus_core::load(&sample_path).expect("failed to load sample network.toml");

    let result = nexus_validate::validate(&network);
    assert!(
        result.is_ok(),
        "sample network should be valid, but got errors: {:?}",
        result.unwrap_err()
    );
}
