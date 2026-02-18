//! Core client crate for the Rust Gemini SDK.

pub mod afc;
mod auth;
pub mod batches;
pub mod caches;
pub mod chats;
pub mod client;
pub mod computer_use;
pub mod deep_research;
pub mod documents;
pub mod error;
pub mod file_search_stores;
pub mod files;
pub mod interactions;
pub mod live;
pub mod live_music;
#[cfg(feature = "mcp")]
pub mod mcp;
mod http_response;
pub mod model_capabilities;
pub mod models;
pub mod operations;
pub mod sse;
pub mod thinking;
pub mod tokenizer;
pub mod tokens;
pub mod tunings;
mod upload;

#[cfg(test)]
mod test_support;

pub use rust_genai_types as types;

pub use client::{Backend, Client, ClientBuilder, Credentials, HttpOptions, VertexConfig};
pub use error::{Error, Result};
