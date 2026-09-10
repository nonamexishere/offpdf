//! Deterministic in-process Fake. Same prompt after the same load yields
//! the same UTF-8. Reserved prompts: `FAIL` (structured error) and `SLOW`
//! (cooperative cancel). No disk model, no child process, no sockets.

use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;

use crate::error::AppError;

use super::{BackendStatus, FakeScript, Health, InferenceBackend};

const BACKEND_ID: &str = "fake";
const SLOW_SLICE: Duration = Duration::from_millis(10);
const SLOW_SLICES: u32 = 500;

pub(crate) struct FakeBackend {
    loaded: AtomicBool,
    cancel: AtomicBool,
    unhealthy: bool,
}

impl FakeBackend {
    pub(super) fn new(script: Option<FakeScript>) -> Self {
        Self {
            loaded: AtomicBool::new(false),
            cancel: AtomicBool::new(false),
            unhealthy: matches!(script, Some(FakeScript::Unhealthy)),
        }
    }

    fn reply(prompt: &str) -> String {
        format!("fake:{prompt}")
    }

    fn take_cancel(&self) -> bool {
        self.cancel.swap(false, Ordering::SeqCst)
    }

    pub fn load(&self) -> Result<(), AppError> {
        self.loaded.store(true, Ordering::SeqCst);
        Ok(())
    }

    pub fn unload(&self) -> Result<(), AppError> {
        self.cancel.store(true, Ordering::SeqCst);
        self.loaded.store(false, Ordering::SeqCst);
        Ok(())
    }

    pub fn generate(&self, prompt: &str) -> Result<String, AppError> {
        if !self.loaded.load(Ordering::SeqCst) {
            return Err(AppError::ai_not_ready());
        }

        if prompt == "FAIL" {
            return Err(AppError::ai_failed());
        }

        if prompt == "SLOW" {
            for _ in 0..SLOW_SLICES {
                if !self.loaded.load(Ordering::SeqCst) {
                    return Err(AppError::ai_not_ready());
                }
                if self.cancel.load(Ordering::SeqCst) {
                    let _ = self.take_cancel();
                    return Err(AppError::cancelled());
                }
                thread::sleep(SLOW_SLICE);
            }
        }

        if !self.loaded.load(Ordering::SeqCst) {
            return Err(AppError::ai_not_ready());
        }
        if self.take_cancel() {
            return Err(AppError::cancelled());
        }

        Ok(Self::reply(prompt))
    }

    pub fn cancel(&self) {
        self.cancel.store(true, Ordering::SeqCst);
    }

    pub fn status(&self) -> BackendStatus {
        if self.loaded.load(Ordering::SeqCst) {
            BackendStatus::Ready
        } else {
            BackendStatus::Unloaded
        }
    }

    pub fn health(&self) -> Health {
        Health {
            ok: self.loaded.load(Ordering::SeqCst) && !self.unhealthy,
            backend_id: BACKEND_ID,
        }
    }
}

impl InferenceBackend for FakeBackend {
    fn load(&self) -> Result<(), AppError> {
        FakeBackend::load(self)
    }

    fn unload(&self) -> Result<(), AppError> {
        FakeBackend::unload(self)
    }

    fn generate(&self, prompt: &str) -> Result<String, AppError> {
        FakeBackend::generate(self, prompt)
    }

    fn cancel(&self) {
        FakeBackend::cancel(self)
    }

    fn status(&self) -> BackendStatus {
        FakeBackend::status(self)
    }

    fn health(&self) -> Health {
        FakeBackend::health(self)
    }
}
