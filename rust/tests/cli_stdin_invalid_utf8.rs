use serde_json::Value;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_DIR: AtomicUsize = AtomicUsize::new(0);

struct Workspace(PathBuf);

impl Workspace {
    fn new() -> Self {
        let serial = NEXT_DIR.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "proselint-rs-stdin-invalid-utf8-{}-{serial}",
            std::process::id()
        ));
        fs::create_dir_all(path.join("home")).expect("create isolated workspace");
        Self(path)
    }

    fn run(&self, config: &str, input: &[u8]) -> std::process::Output {
        fs::write(self.0.join("config.json"), config).expect("write selected-rule config");
        let mut command = Command::new(env!("CARGO_BIN_EXE_proselint-rs"));
        command
            .current_dir(&self.0)
            .env("HOME", self.0.join("home"))
            .env_remove("XDG_CONFIG_HOME")
            .args([
                "check",
                "--config",
                "config.json",
                "--output-format",
                "json",
            ]);
        let mut child = command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("run native CLI");
        child
            .stdin
            .take()
            .unwrap()
            .write_all(input)
            .expect("write stdin bytes");
        child.wait_with_output().expect("wait for native CLI")
    }
}

impl Drop for Workspace {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn error_config() -> &'static str {
    r#"{"max_errors":1000,"checks":{"misc":{"apologizing":true}}}"#
}

fn fancy_config() -> &'static str {
    r#"{"max_errors":1000,"checks":{"lexical_illusions":true}}"#
}

fn disabled_config() -> &'static str {
    r#"{"checks":{}}"#
}

#[test]
fn fast_rule_reports_python_surrogateescape_errors_at_each_byte_position() {
    for (input, position) in [
        (&b"\xffOur policy states: More research is needed."[..], 1),
        (&b"Our policy states: Mo\xffre research is needed."[..], 22),
        (&b"Our policy states: More research is needed.\xff"[..], 44),
    ] {
        let output = Workspace::new().run(error_config(), input);
        assert_eq!(output.status.code(), Some(0), "stderr: {:?}", output.stderr);
        assert!(output.stderr.is_empty(), "stderr: {:?}", output.stderr);
        let value: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(value["result"]["<stdin>"]["error"]["code"], -31999);
        assert_eq!(
            value["result"]["<stdin>"]["error"]["data"],
            "UnicodeEncodeError"
        );
        assert_eq!(
            value["result"]["<stdin>"]["error"]["message"],
            format!(
                "'utf-8' codec can't encode character '\\udcff' in position {position}: surrogates not allowed"
            )
        );
    }
}

#[test]
fn fancy_rule_matches_around_invalid_bytes_and_breaks_inside_them() {
    let before = Workspace::new().run(fancy_config(), b"\xffword word");
    assert_eq!(before.status.code(), Some(1), "stderr: {:?}", before.stderr);
    let before_json: Value = serde_json::from_slice(&before.stdout).unwrap();
    assert_eq!(
        before_json["result"]["<stdin>"]["diagnostics"][0]["span"],
        serde_json::json!([2, 11])
    );

    let after = Workspace::new().run(fancy_config(), b"word word\xff");
    assert_eq!(after.status.code(), Some(1), "stderr: {:?}", after.stderr);
    let after_json: Value = serde_json::from_slice(&after.stdout).unwrap();
    assert_eq!(
        after_json["result"]["<stdin>"]["diagnostics"][0]["span"],
        serde_json::json!([1, 10])
    );

    let inside = Workspace::new().run(fancy_config(), b"word\xff word");
    assert_eq!(inside.status.code(), Some(0), "stderr: {:?}", inside.stderr);
    let inside_json: Value = serde_json::from_slice(&inside.stdout).unwrap();
    assert_eq!(
        inside_json["result"]["<stdin>"]["diagnostics"],
        serde_json::json!([])
    );
}

#[test]
fn invalid_bytes_preserve_codepoint_positions_and_dispatch_limits() {
    let multibyte = Workspace::new().run(fancy_config(), b"\xc3\xa9\xffword word");
    assert_eq!(
        multibyte.status.code(),
        Some(1),
        "stderr: {:?}",
        multibyte.stderr
    );
    let multibyte_json: Value = serde_json::from_slice(&multibyte.stdout).unwrap();
    assert_eq!(
        multibyte_json["result"]["<stdin>"]["diagnostics"][0]["pos"],
        serde_json::json!([1, 3])
    );
    assert_eq!(
        multibyte_json["result"]["<stdin>"]["diagnostics"][0]["span"],
        serde_json::json!([3, 12])
    );

    let consecutive = Workspace::new().run(
        error_config(),
        b"\xff\xfeOur policy states: More research is needed.",
    );
    assert_eq!(
        consecutive.status.code(),
        Some(0),
        "stderr: {:?}",
        consecutive.stderr
    );
    let consecutive_json: Value = serde_json::from_slice(&consecutive.stdout).unwrap();
    assert_eq!(
        consecutive_json["result"]["<stdin>"]["error"]["message"],
        "'utf-8' codec can't encode characters in position 1-2: surrogates not allowed"
    );

    let mixed = |max_errors| {
        Workspace::new().run(
            &format!(
                r#"{{"max_errors":{max_errors},"checks":{{"lexical_illusions":true,"misc":{{"apologizing":true}}}}}}"#
            ),
            b"\xffword word\nOur policy states: More research is needed.",
        )
    };
    let zero = mixed(0);
    assert_eq!(zero.status.code(), Some(0), "stderr: {:?}", zero.stderr);
    let zero_json: Value = serde_json::from_slice(&zero.stdout).unwrap();
    assert_eq!(
        zero_json["result"]["<stdin>"]["diagnostics"],
        serde_json::json!([])
    );

    let one = mixed(1);
    assert_eq!(one.status.code(), Some(1), "stderr: {:?}", one.stderr);
    let one_json: Value = serde_json::from_slice(&one.stdout).unwrap();
    assert_eq!(
        one_json["result"]["<stdin>"]["diagnostics"][0]["span"],
        serde_json::json!([2, 11])
    );
    assert!(one_json["result"]["<stdin>"].get("error").is_none());
}

#[test]
fn valid_unicode_and_crlf_controls_remain_valid_stdin() {
    for input in ["�".as_bytes(), "\u{e0ff}".as_bytes(), b"alpha\r\nbeta"] {
        let output = Workspace::new().run(disabled_config(), input);
        assert_eq!(output.status.code(), Some(0), "stderr: {:?}", output.stderr);
        assert!(output.stderr.is_empty(), "stderr: {:?}", output.stderr);
        let value: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(
            value["result"]["<stdin>"]["diagnostics"],
            serde_json::json!([])
        );
    }
}
