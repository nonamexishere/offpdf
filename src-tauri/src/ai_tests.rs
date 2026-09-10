//! Unit tests for `crate::ai` InferenceBackend + Fake (issue #81).
//!
//! Public names are the Approach A lock: `ai::fake` / `ai::fake_with_script`,
//! `InferenceBackend` {load, unload, generate, cancel, status, health},
//! reserved prompts `FAIL` and `SLOW`, `FakeScript::Unhealthy`.

use std::path::{Path, PathBuf};
use std::sync::Arc;

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn ai_src_dir() -> PathBuf {
    manifest_dir().join("src/ai")
}

fn walk_rs_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(_) => return,
    };
    for entry in entries {
        let path = match entry {
            Ok(entry) => entry.path(),
            Err(_) => continue,
        };
        if path.is_dir() {
            walk_rs_files(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            out.push(path);
        }
    }
}

fn default_dependency_names(toml: &str) -> Vec<String> {
    let mut names = Vec::new();
    let mut in_deps = false;
    for line in toml.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            in_deps = trimmed == "[dependencies]";
            continue;
        }
        if !in_deps || trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if let Some(eq) = trimmed.find('=') {
            let name = trimmed[..eq].trim();
            if !name.is_empty() {
                names.push(name.to_string());
            }
        }
    }
    names
}

fn is_banned_runtime_crate(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    lower.contains("llama")
        || lower.contains("candle")
        || lower.contains("reqwest")
        || lower == "ort"
        || lower.starts_with("ort-")
}

fn assert_ai_error_fields(err: &crate::error::AppError, must_id: &str) {
    assert!(
        !err.title.is_empty(),
        "{must_id}: AppError.title must be non-empty"
    );
    assert!(
        !err.message.is_empty(),
        "{must_id}: AppError.message must be non-empty"
    );
}

#[test]
fn ai_cargo_no_runtime_crate() {
    let toml_path = manifest_dir().join("Cargo.toml");
    let toml = std::fs::read_to_string(&toml_path)
        .unwrap_or_else(|err| panic!("ai-cargo-no-runtime-crate: read {}: {err}", toml_path.display()));
    let banned: Vec<String> = default_dependency_names(&toml)
        .into_iter()
        .filter(|name| is_banned_runtime_crate(name))
        .collect();
    assert!(
        banned.is_empty(),
        "ai-cargo-no-runtime-crate: default [dependencies] must not add llama / candle / ort / reqwest; found {banned:?}"
    );
}

#[test]
fn ai_no_network_no_shell() {
    let ai_dir = ai_src_dir();
    assert!(
        ai_dir.is_dir(),
        "ai-no-network-no-shell: src-tauri/src/ai/ must exist so the source lock can scan it"
    );

    let mut files = Vec::new();
    walk_rs_files(&ai_dir, &mut files);
    assert!(
        !files.is_empty(),
        "ai-no-network-no-shell: src-tauri/src/ai/ must contain Rust sources to scan"
    );

    const FORBIDDEN: &[&str] = &[
        "reqwest",
        "ureq",
        "std::net",
        "std::process::Command",
        "XAI_API_KEY",
        "api.openai.com",
        "api.x.ai",
        "api.anthropic.com",
        "generativelanguage.googleapis.com",
        "api.groq.com",
    ];

    let mut hits = Vec::new();
    for path in &files {
        let src = std::fs::read_to_string(path).unwrap_or_else(|err| {
            panic!("ai-no-network-no-shell: read {}: {err}", path.display())
        });
        for token in FORBIDDEN {
            if src.contains(token) {
                hits.push(format!("{}: {token}", path.display()));
            }
        }
    }
    assert!(
        hits.is_empty(),
        "ai-no-network-no-shell: src-tauri/src/ai/** must not contain network, shell, or cloud-host tokens; found {hits:?}"
    );
}

#[test]
fn ai_no_pdf_bytes_api() {
    let backend = crate::ai::fake();
    backend
        .load()
        .expect("ai-no-pdf-bytes-api: load must succeed without a model file");
    let prompt: &str = "utf-8 prompt, not pdf bytes";
    let out = backend
        .generate(prompt)
        .expect("ai-no-pdf-bytes-api: generate must take a string prompt");
    let _utf8: &str = out.as_str();

    let ai_dir = ai_src_dir();
    assert!(
        ai_dir.is_dir(),
        "ai-no-pdf-bytes-api: src-tauri/src/ai/ must exist so generate signatures can be scanned"
    );
    let mut files = Vec::new();
    walk_rs_files(&ai_dir, &mut files);
    let mut saw_generate = false;
    let mut bad = Vec::new();
    for path in &files {
        let src = std::fs::read_to_string(path).unwrap_or_else(|err| {
            panic!("ai-no-pdf-bytes-api: read {}: {err}", path.display())
        });
        let mut rest = src.as_str();
        while let Some(idx) = rest.find("fn generate") {
            saw_generate = true;
            let after = &rest[idx..];
            let end = after.find('{').or_else(|| after.find(';')).unwrap_or(after.len());
            let sig = &after[..end];
            for token in ["Vec<u8>", "&[u8]", "PathBuf", "std::path::Path", "DynamicImage"] {
                if sig.contains(token) {
                    bad.push(format!("{}: {token}", path.display()));
                }
            }
            rest = &after["fn generate".len()..];
        }
    }
    assert!(
        saw_generate,
        "ai-no-pdf-bytes-api: no generate signature to inspect under src-tauri/src/ai/"
    );
    assert!(
        bad.is_empty(),
        "ai-no-pdf-bytes-api: generate must not take PDF/page bytes or a path; found {bad:?}"
    );
}

#[test]
fn ai_trait_surface() {
    let backend = crate::ai::fake();
    let _ = backend.status();
    let _ = backend.health();
    backend
        .load()
        .expect("ai-trait-surface: load must exist and not fail on Fake");
    let _ = backend.generate("surface");
    let _ = backend.cancel();
    backend
        .unload()
        .expect("ai-trait-surface: unload must exist and not fail on Fake");
}

#[test]
fn ai_factory_hides_concrete() {
    let backend = crate::ai::fake();
    let _ = backend.status();
    let scripted = crate::ai::fake_with_script(crate::ai::FakeScript::Unhealthy);
    let _ = scripted.health();

    let ai_dir = ai_src_dir();
    assert!(
        ai_dir.is_dir(),
        "ai-factory-hides-concrete: src-tauri/src/ai/ must exist so the source lock can scan it"
    );
    let mut files = Vec::new();
    walk_rs_files(&ai_dir, &mut files);
    assert!(
        !files.is_empty(),
        "ai-factory-hides-concrete: src-tauri/src/ai/ must contain Rust sources to scan"
    );
    let mut hits = Vec::new();
    for path in &files {
        let src = std::fs::read_to_string(path).unwrap_or_else(|err| {
            panic!("ai-factory-hides-concrete: read {}: {err}", path.display())
        });
        for line in src.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("pub ")
                && (trimmed.contains("llama")
                    || trimmed.contains("candle")
                    || trimmed.contains("ort::"))
            {
                hits.push(format!("{}: {trimmed}", path.display()));
            }
        }
    }
    assert!(
        hits.is_empty(),
        "ai-factory-hides-concrete: ai/** must not pub a llama/candle/ort type; found {hits:?}"
    );
}

#[test]
fn ai_fake_deterministic() {
    let backend = crate::ai::fake();
    backend
        .load()
        .expect("ai-fake-deterministic: load must succeed without a model file");
    let prompt = "same prompt twice";
    let first = backend
        .generate(prompt)
        .expect("ai-fake-deterministic: first generate");
    let second = backend
        .generate(prompt)
        .expect("ai-fake-deterministic: second generate");
    assert_eq!(
        first, second,
        "ai-fake-deterministic: same load + same prompt must yield identical UTF-8"
    );
    assert_eq!(
        first.as_bytes(),
        second.as_bytes(),
        "ai-fake-deterministic: outputs must be byte-identical"
    );
}

#[test]
fn ai_fake_not_ready() {
    let backend = crate::ai::fake();
    let err = backend
        .generate("hello")
        .expect_err("ai-fake-not-ready: generate before load must return AppError, not Ok");
    assert_eq!(
        err.code, "AI_NOT_READY",
        "ai-fake-not-ready: .code must be the stable AI_NOT_READY"
    );
    assert!(
        err.code.starts_with("AI_"),
        "ai-fake-not-ready: .code must be an AI_* code, got {}",
        err.code
    );
    assert_ai_error_fields(&err, "ai-fake-not-ready");
}

#[test]
fn ai_fake_fail_apperror() {
    let backend = crate::ai::fake();
    backend
        .load()
        .expect("ai-fake-fail-apperror: load must succeed before FAIL");
    let err = backend
        .generate("FAIL")
        .expect_err("ai-fake-fail-apperror: reserved prompt FAIL must return AppError");
    assert_eq!(
        err.code, "AI_FAILED",
        "ai-fake-fail-apperror: .code must be AI_FAILED, got {}",
        err.code
    );
    assert!(
        err.code.starts_with("AI_"),
        "ai-fake-fail-apperror: .code must be AI_*, got {}",
        err.code
    );
    assert_ne!(
        err.code, "ENGINE_FAILED",
        "ai-fake-fail-apperror: must not reuse the PDF-engine code"
    );
    assert_ai_error_fields(&err, "ai-fake-fail-apperror");
}

#[test]
fn ai_fake_cancel_cancelled() {
    let backend = Arc::new(crate::ai::fake());
    backend
        .load()
        .expect("ai-fake-cancel-cancelled: load must succeed before SLOW");

    let worker = {
        let backend = Arc::clone(&backend);
        std::thread::spawn(move || backend.generate("SLOW"))
    };

    let _ = backend.cancel();

    let result = worker
        .join()
        .expect("ai-fake-cancel-cancelled: generate thread must not panic");
    let err = result.expect_err("ai-fake-cancel-cancelled: cancelled SLOW must be Err");
    assert_eq!(
        err.code,
        crate::error::AppError::cancelled().code,
        "ai-fake-cancel-cancelled: must reuse AppError::cancelled"
    );
    assert_eq!(
        err.code, "CANCELLED",
        "ai-fake-cancel-cancelled: .code must be CANCELLED"
    );
    assert_ne!(
        err.code, "ENGINE_FAILED",
        "ai-fake-cancel-cancelled: must not reuse ENGINE_FAILED"
    );
}

#[test]
fn ai_fake_cancel_unload() {
    let backend = Arc::new(crate::ai::fake());
    backend
        .load()
        .expect("ai-fake-cancel-unload: load must succeed before SLOW");

    let worker = {
        let backend = Arc::clone(&backend);
        std::thread::spawn(move || backend.generate("SLOW"))
    };

    let _ = backend.cancel();
    backend
        .unload()
        .expect("ai-fake-cancel-unload: unload after cancel must succeed");

    let result = worker
        .join()
        .expect("ai-fake-cancel-unload: generate thread must not panic");
    let err = result.expect_err(
        "ai-fake-cancel-unload: cancel then unload during SLOW must be Err, never Ok",
    );
    assert!(
        err.code == "CANCELLED" || err.code == "AI_NOT_READY",
        "ai-fake-cancel-unload: .code must be CANCELLED or AI_NOT_READY, got {}",
        err.code
    );
}

#[test]
fn ai_fake_health_offline() {
    let backend = crate::ai::fake();
    backend
        .load()
        .expect("ai-fake-health-offline: load must succeed without a model file");
    let health = backend.health();
    assert!(
        health.ok,
        "ai-fake-health-offline: health after load must be ok without a model file"
    );
    assert_eq!(
        health.backend_id, "fake",
        "ai-fake-health-offline: backend id must be \"fake\""
    );

    let unhealthy = crate::ai::fake_with_script(crate::ai::FakeScript::Unhealthy);
    unhealthy
        .load()
        .expect("ai-fake-health-offline: scripted unhealthy load must not need a model");
    let health = unhealthy.health();
    assert!(
        !health.ok,
        "ai-fake-health-offline: FakeScript::Unhealthy must report not-ok without a model or network"
    );
}

#[test]
fn ai_fake_unload() {
    let backend = crate::ai::fake();
    backend
        .load()
        .expect("ai-fake-unload: load must succeed without a model file");
    assert!(
        matches!(backend.status(), crate::ai::BackendStatus::Ready),
        "ai-fake-unload: status after load must be Ready"
    );

    backend
        .unload()
        .expect("ai-fake-unload: unload must succeed");
    assert!(
        matches!(backend.status(), crate::ai::BackendStatus::Unloaded),
        "ai-fake-unload: status after unload must be Unloaded"
    );

    let err = backend
        .generate("after unload")
        .expect_err("ai-fake-unload: generate after unload must return AppError");
    assert_eq!(
        err.code, "AI_NOT_READY",
        "ai-fake-unload: generate after unload must be AI_NOT_READY"
    );
    assert_ai_error_fields(&err, "ai-fake-unload");
}
