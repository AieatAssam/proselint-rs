//! Native Rust implementation of Proselint.

use serde_json::{Map, Value, json};
use std::path::Path;

/// Format an absolute local path as a `file:` URI, escaping bytes as Python's
/// `Path.as_uri()` does for the ASCII paths supported by the CLI.
pub fn file_uri(path: &Path) -> Result<String, std::io::Error> {
    let absolute = path.canonicalize()?;
    let bytes = absolute.to_string_lossy();
    let mut uri = String::from("file://");
    for byte in bytes.as_bytes() {
        if byte.is_ascii_alphanumeric() || b"/-._~:".contains(byte) {
            uri.push(char::from(*byte));
        } else {
            use std::fmt::Write as _;
            write!(&mut uri, "%{byte:02X}").expect("writing to String cannot fail");
        }
    }
    Ok(uri)
}

/// Construct the JSON result envelope for a source with no diagnostics.
pub fn clean_json_result(source: &str) -> Value {
    let mut source_results = Map::new();
    source_results.insert(source.to_owned(), json!({ "diagnostics": [] }));
    json!({ "result": source_results })
}
