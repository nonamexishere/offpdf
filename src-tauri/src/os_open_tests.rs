//! Unit tests for `crate::os_open` (issue #74).
//!
//! Named helper: `src-tauri/src/os_open.rs`. The product module may be missing
//! until impl. Expected public surface:
//!
//! - `parse_opened_argv(args: impl IntoIterator<Item = impl AsRef<str>>) -> Vec<PathBuf>`
//!   skips argv[0], then maps the rest through `parse_opened_token`.
//! - `parse_opened_token(token: &str) -> Option<PathBuf>`
//!   accepts a raw path or `file://` (percent-decoded); drops `-flag`s and
//!   non-file URLs. Does not read file bytes.

use std::path::PathBuf;

fn argv(args: &[&str]) -> Vec<PathBuf> {
    let owned: Vec<String> = args.iter().map(|s| (*s).to_string()).collect();
    crate::os_open::parse_opened_argv(owned)
}

fn token(s: &str) -> Option<PathBuf> {
    crate::os_open::parse_opened_token(s)
}

#[test]
fn os_open_parse_argv_plain() {
    assert_eq!(
        argv(&["offpdf", "/tmp/a.pdf"]),
        vec![PathBuf::from("/tmp/a.pdf")],
        "os-open-parse-argv-plain: /tmp/a.pdf must become an opened path",
    );
    assert_eq!(
        argv(&[
            "/Applications/OffPDF.app/Contents/MacOS/offpdf",
            "/tmp/a.pdf",
            "/tmp/b.png",
        ]),
        vec![PathBuf::from("/tmp/a.pdf"), PathBuf::from("/tmp/b.png")],
        "os-open-parse-argv-plain: every trailing path must be kept",
    );
}

#[test]
fn os_open_parse_spaces_unicode() {
    assert_eq!(
        token("/tmp/ünicode 报告.pdf"),
        Some(PathBuf::from("/tmp/ünicode 报告.pdf")),
        "os-open-parse-spaces-unicode: raw path with spaces and non-ASCII must survive",
    );
    assert_eq!(
        token("file:///tmp/my%20file.pdf"),
        Some(PathBuf::from("/tmp/my file.pdf")),
        "os-open-parse-spaces-unicode: file:// percent-decode must yield /tmp/my file.pdf",
    );
    assert_eq!(
        token("file:///tmp/%C3%BCnicode%20%E6%8A%A5%E5%91%8A.pdf"),
        Some(PathBuf::from("/tmp/ünicode 报告.pdf")),
        "os-open-parse-spaces-unicode: file:// percent-decode must yield /tmp/ünicode 报告.pdf",
    );
}

#[test]
fn os_open_parse_skip_flags_and_urls() {
    assert_eq!(
        argv(&[
            "offpdf",
            "-flag",
            "--help",
            "https://example.com/a.pdf",
            "/tmp/a.pdf",
        ]),
        vec![PathBuf::from("/tmp/a.pdf")],
        "os-open-parse-skip-flags-and-urls: drop argv[0], -flag, and https://",
    );
    assert_eq!(
        token("-flag"),
        None,
        "os-open-parse-skip-flags-and-urls: -flag is not an opened path",
    );
    assert_eq!(
        token("https://example.com/a.pdf"),
        None,
        "os-open-parse-skip-flags-and-urls: https:// is not an opened path",
    );
    assert_eq!(
        token("offpdf"),
        None,
        "os-open-parse-skip-flags-and-urls: a bare argv[0]-like token is not an opened path",
    );
}
