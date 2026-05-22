//! FFI surface for embedding `attune-core` in other languages.
//!
//! This module will host the UniFFI-friendly type wrappers that cross the
//! Swift / iOS / mobile boundary. It is intentionally empty for now; the
//! plan is to land the audio-capture FFI shape here in the same PR that
//! wires up the Swift app shell. Keeping the module in place so its path
//! (`attune_core::ffi`) is stable for downstream consumers ahead of that
//! work.
