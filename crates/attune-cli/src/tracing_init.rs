//! Single-call `tracing-subscriber` setup for the CLI binary.

use tracing_subscriber::EnvFilter;

/// Initialise the global tracing subscriber. Reads `RUST_LOG` for the
/// filter, falling back to a sensible default that silences cpal's
/// per-frame chatter while keeping the library's info-level messages.
pub fn init_tracing() {
    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info,cpal=warn"));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .compact()
        .init();
}
