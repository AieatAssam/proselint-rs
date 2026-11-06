use serde_json::{Value, json};
use std::fs;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_DIR: AtomicUsize = AtomicUsize::new(0);

struct Workspace(PathBuf);

impl Workspace {
    fn new() -> Self {
        let serial = NEXT_DIR.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "proselint-rs-malformed-config-{}-{serial}",
            std::process::id()
        ));
        fs::create_dir(&path).expect("create isolated test workspace");
        Self(path)
    }
}

impl Drop for Workspace {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn assert_oracle_error(config: &[u8], message: &str) {
    assert_structured_error(config, "JSONDecodeError", message);
}

fn assert_structured_error(config: &[u8], data: &str, message: &str) {
    let workspace = Workspace::new();
    fs::write(workspace.0.join("malformed config.json"), config)
        .expect("write malformed config fixture");
    let output = Command::new(env!("CARGO_BIN_EXE_proselint"))
        .current_dir(&workspace.0)
        .args([
            "check",
            "--config",
            "malformed config.json",
            "--output-format",
            "json",
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("run native CLI with malformed config");
    assert_eq!(output.status.code(), Some(4), "stderr: {:?}", output.stderr);
    let value: Value = serde_json::from_slice(&output.stdout).unwrap_or(Value::Null);
    assert_eq!(
        value,
        json!({
            "error": {
                "code": -31999,
                "data": data,
                "message": message
            }
        })
    );
    assert!(output.stderr.is_empty(), "stderr: {:?}", output.stderr);
}

#[test]
fn empty_config_matches_pinned_json_decode_error() {
    assert_oracle_error(b"", "Expecting value: line 1 column 1 (char 0)");
}

#[test]
fn trailing_comma_matches_pinned_json_decode_error() {
    assert_oracle_error(
        br#"{"checks":{},}"#,
        "Expecting property name enclosed in double quotes: line 1 column 14 (char 13)",
    );
}

#[test]
fn invalid_escape_matches_pinned_json_decode_error() {
    assert_oracle_error(
        br#"{"checks":{"x":"\q"}}"#,
        "Invalid \\escape: line 1 column 17 (char 16)",
    );
}

#[test]
fn unterminated_string_matches_pinned_json_decode_error() {
    assert_oracle_error(
        br#"{"checks":{"x":"oops}}"#,
        "Unterminated string starting at: line 1 column 16 (char 15)",
    );
}

#[test]
fn extra_data_matches_pinned_json_decode_error() {
    assert_oracle_error(b"{} {}", "Extra data: line 1 column 4 (char 3)");
}

#[test]
fn missing_colon_matches_pinned_json_decode_error() {
    assert_oracle_error(
        br#"{"checks" {}}"#,
        "Expecting ':' delimiter: line 1 column 11 (char 10)",
    );
}

#[test]
fn leading_zero_number_matches_pinned_json_decode_error() {
    assert_oracle_error(
        br#"{"max_errors":01}"#,
        "Expecting ',' delimiter: line 1 column 16 (char 15)",
    );
}

#[test]
fn multibyte_multiline_error_uses_pinned_character_coordinates() {
    assert_oracle_error(
        "{\n  \"checks\": {\"é\": true,\n  }\n}".as_bytes(),
        "Expecting property name enclosed in double quotes: line 3 column 3 (char 28)",
    );
}

#[test]
fn crlf_config_newlines_match_pinned_decode_error() {
    assert_oracle_error(
        b"{\r\n  \"checks\": ,\r\n}",
        "Expecting value: line 2 column 13 (char 14)",
    );
}

#[test]
fn incomplete_exponent_matches_pinned_decode_error() {
    assert_oracle_error(
        br#"{"max_errors":1e}"#,
        "Expecting ',' delimiter: line 1 column 16 (char 15)",
    );
}

#[test]
fn incomplete_unicode_escape_matches_pinned_decode_error() {
    assert_oracle_error(
        br#"{"checks":"\u12"}"#,
        "Invalid \\uXXXX escape: line 1 column 13 (char 12)",
    );
}

#[test]
fn array_trailing_comma_matches_pinned_decode_error() {
    assert_oracle_error(b"[1,]", "Expecting value: line 1 column 4 (char 3)");
}

#[test]
fn literal_control_character_matches_pinned_decode_error() {
    assert_oracle_error(
        b"{\"x\":\"a\x01b\"}",
        "Invalid control character at: line 1 column 8 (char 7)",
    );
}

#[test]
fn utf8_bom_matches_pinned_decode_error() {
    assert_oracle_error(
        b"\xef\xbb\xbf{}",
        "Unexpected UTF-8 BOM (decode using utf-8-sig): line 1 column 1 (char 0)",
    );
}

#[test]
fn leading_newline_matches_pinned_decode_error() {
    assert_oracle_error(
        b"\n{",
        "Expecting property name enclosed in double quotes: line 2 column 2 (char 2)",
    );
}

#[test]
fn nonstandard_constants_continue_to_trailing_comma_error() {
    assert_oracle_error(
        b"{\"x\":[NaN,Infinity,-Infinity],}",
        "Expecting property name enclosed in double quotes: line 1 column 31 (char 30)",
    );
}

#[test]
fn surrogate_escape_continues_to_trailing_comma_error() {
    assert_oracle_error(
        br#"{"x":"\ud800",}"#,
        "Expecting property name enclosed in double quotes: line 1 column 15 (char 14)",
    );
}

#[test]
fn long_integer_continues_to_trailing_comma_error() {
    let mut config = b"{\"x\":".to_vec();
    config.extend(std::iter::repeat_n(b'1', 400));
    config.extend_from_slice(b",}");
    assert_oracle_error(
        &config,
        "Expecting property name enclosed in double quotes: line 1 column 407 (char 406)",
    );
}

#[test]
fn deep_arrays_continue_to_trailing_comma_error() {
    let mut config = b"{\"x\":".to_vec();
    config.extend(std::iter::repeat_n(b'[', 150));
    config.push(b'0');
    config.extend(std::iter::repeat_n(b']', 150));
    config.extend_from_slice(b",}");
    assert_oracle_error(
        &config,
        "Expecting property name enclosed in double quotes: line 1 column 308 (char 307)",
    );
}

#[test]
fn partial_literal_matches_pinned_decode_error() {
    assert_oracle_error(b"{\"x\":tru}", "Expecting value: line 1 column 6 (char 5)");
}

#[test]
fn invalid_minus_matches_pinned_decode_error() {
    assert_oracle_error(b"{\"x\":-}", "Expecting value: line 1 column 6 (char 5)");
}

#[test]
fn root_incomplete_number_matches_pinned_decode_error() {
    assert_oracle_error(b"1.", "Extra data: line 1 column 2 (char 1)");
}

#[test]
fn invalid_utf8_lead_85_matches_pinned_decode_error() {
    assert_structured_error(
        &[0x7b, 0x22, 0x85, 0x22, 0x7d],
        "UnicodeDecodeError",
        "'utf-8' codec can't decode byte 0x85 in position 2: invalid start byte",
    );
}

#[test]
fn invalid_utf8_lead_ff_matches_pinned_decode_error() {
    assert_structured_error(
        &[0x7b, 0x22, 0x78, 0x22, 0x3a, 0x22, 0xff, 0x22, 0x7d],
        "UnicodeDecodeError",
        "'utf-8' codec can't decode byte 0xff in position 6: invalid start byte",
    );
}

#[test]
fn invalid_utf8_continuation_matches_pinned_decode_error() {
    assert_structured_error(
        &[0x7b, 0x22, 0x78, 0x22, 0x3a, 0x22, 0xc3, 0x28, 0x22, 0x7d],
        "UnicodeDecodeError",
        "'utf-8' codec can't decode byte 0xc3 in position 6: invalid continuation byte",
    );
}

#[test]
fn truncated_utf8_two_byte_sequence_matches_pinned_decode_error() {
    assert_structured_error(
        &[0x7b, 0x22, 0x78, 0x22, 0x3a, 0x22, 0xc3],
        "UnicodeDecodeError",
        "'utf-8' codec can't decode byte 0xc3 in position 6: unexpected end of data",
    );
}

#[test]
fn truncated_utf8_four_byte_sequence_matches_pinned_decode_error() {
    assert_structured_error(
        &[0x7b, 0x22, 0x78, 0x22, 0x3a, 0x22, 0xf0, 0x9f],
        "UnicodeDecodeError",
        "'utf-8' codec can't decode bytes in position 6-7: unexpected end of data",
    );
}
