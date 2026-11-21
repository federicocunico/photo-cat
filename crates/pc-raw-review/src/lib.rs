//! Varianti RAW (Fujifilm X-T5): revisione delle varianti di sviluppo dei RAF.
pub mod export;
pub mod metadata;
pub mod preview;
pub mod scan;
pub mod session;

#[cfg(any(windows, feature = "fake-camera"))]
pub mod camera;
#[cfg(windows)]
pub mod variants;
