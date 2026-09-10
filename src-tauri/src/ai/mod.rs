//! Isolation layer for on-device inference.
//!
//! Generate is UTF-8 prompt in, UTF-8 text out. No model file.
//! Methods take `&self` so `cancel` can race `generate`.

mod fake;

use crate::error::AppError;

use fake::FakeBackend;

/// Load / unload / generate / cancel / status / health. Methods take `&self`
/// so `cancel` can run from another thread while `generate` is in flight.
#[allow(dead_code)]
pub trait InferenceBackend: Send + Sync {
    fn load(&self) -> Result<(), AppError>;
    fn unload(&self) -> Result<(), AppError>;
    fn generate(&self, prompt: &str) -> Result<String, AppError>;
    fn cancel(&self);
    fn status(&self) -> BackendStatus;
    fn health(&self) -> Health;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendStatus {
    Unloaded,
    Ready,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Health {
    pub ok: bool,
    pub backend_id: &'static str,
}

/// Scripted Fake behaviour set at construction. No model file, no download.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FakeScript {
    Unhealthy,
}

/// In-process Fake. No filesystem path, no download.
pub(crate) fn fake() -> FakeBackend {
    FakeBackend::new(None)
}

/// In-process Fake with a scripted health path.
pub(crate) fn fake_with_script(script: FakeScript) -> FakeBackend {
    FakeBackend::new(Some(script))
}
