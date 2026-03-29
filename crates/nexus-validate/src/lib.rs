pub mod error;
pub mod rules;

pub use error::ValidationError;
use nexus_core::Network;

/// Validate a resolved network. Returns Ok(()) if valid, or a Vec of all errors found.
/// All rules are applied and every error is collected before returning — no short-circuiting.
pub fn validate(network: &Network) -> Result<(), Vec<ValidationError>> {
    let mut errors = Vec::new();

    rules::check_unmatched_sends(network, &mut errors);
    rules::check_unmatched_receives(network, &mut errors);
    rules::check_orphan_nodes(network, &mut errors);
    rules::check_iceoryx_pod(network, &mut errors);
    rules::check_duplicate_names(network, &mut errors);

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}
