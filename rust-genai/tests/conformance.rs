//! Conformance contract suite for compatibility-matrix claims.
//!
//! Marker semantics:
//! - `mock_*`: no credentials required, runs in regular CI.
//! - `live_gemini_*`: requires `GEMINI_API_KEY`, runs manually or in nightly.
//! - `live_vertex_*`: requires `GOOGLE_CLOUD_PROJECT`,
//!   `GOOGLE_CLOUD_LOCATION`, and ADC credentials, runs manually or in nightly.
//! - `preview_*`: reserved for preview-only paths and release-informational
//!   probes.
//! - `expensive_*`: opt-in manual probes for higher-cost surfaces.

#[path = "conformance/support.rs"]
mod support;

#[path = "conformance/gemini_files.rs"]
mod gemini_files;
#[path = "conformance/gemini_models.rs"]
mod gemini_models;
#[path = "conformance/gemini_streaming.rs"]
mod gemini_streaming;
#[path = "conformance/json_generation.rs"]
mod json_generation;
#[path = "conformance/retry_policy.rs"]
mod retry_policy;
#[path = "conformance/vertex_guards.rs"]
mod vertex_guards;
#[path = "conformance/vertex_models.rs"]
mod vertex_models;
#[path = "conformance/vertex_streaming.rs"]
mod vertex_streaming;
