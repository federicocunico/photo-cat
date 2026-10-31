//! Modelli neurali di photo-cat: manifest, verifica, download, runtime ONNX e self-test.

pub mod download;
pub mod manifest;
pub mod pool;
pub mod runtime;
pub mod store;

pub use manifest::{Entry, Manifest, Role};
