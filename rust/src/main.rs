use clap::{Parser, Subcommand, ValueEnum};
use proselint_rs::{
    airlinese_diagnostics, am_pm_lowercase_periods_diagnostics, am_pm_spacing_diagnostics,
    annotation_diagnostics_in_matcher_order, apologizing_diagnostics, back_formations_diagnostics,
    bureaucratese_diagnostics, but_diagnostics, capitalization_days_diagnostics,
    capitalization_months_diagnostics, capitalization_seasons_diagnostics,
    capitalization_terms_diagnostics, cliches_hell_diagnostics,
    composition_diagnostics_in_matcher_order, currency_diagnostics, debased_diagnostics,
    eponymous_adjectives_diagnostics, false_plurals_kudos_diagnostics,
    false_plurals_misc_diagnostics_in_matcher_order, file_uri, greylist_diagnostics,
    hedging_diagnostics, illogic_coin_diagnostics, illogic_collusion_diagnostics,
    inferior_superior_diagnostics, institution_name_diagnostics, jargon_diagnostics, json_result,
    latin_diagnostics, lie_detector_diagnostics, many_a_diagnostics, mental_telepathy_diagnostics,
    metadiscourse_diagnostics, normalize_file_newlines, p_equals_zero_diagnostics,
    phrasal_adjectives_ly_diagnostics, pretension_diagnostics, professions_diagnostics,
    spelling_ally_ly_diagnostics, spelling_ance_ence_diagnostics, spelling_ely_ly_diagnostics,
    suddenly_diagnostics, typography_copyright_diagnostics, typography_ellipsis_diagnostics,
    typography_multiplication_diagnostics, typography_sentence_spacing_diagnostics,
    typography_trademark_diagnostics, wallace_diagnostics, whence_diagnostics,
};
use serde::Deserialize;
use serde_json::Value;
use std::fs;
use std::io::{self, Read};
use std::path::PathBuf;
use std::process::ExitCode;

#[derive(Debug)]
struct JsonSyntaxError {
    reason: &'static str,
    offset: usize,
}

#[derive(Debug)]
enum JsonScanError {
    Syntax(JsonSyntaxError),
    Recursion(&'static str),
}

#[derive(Clone, Copy)]
enum JsonFrame {
    Object(ObjectState),
    Array(ArrayState),
}

#[derive(Clone, Copy)]
enum ObjectState {
    KeyOrEnd,
    Key,
    Colon,
    Value,
    CommaOrEnd,
}

#[derive(Clone, Copy)]
enum ArrayState {
    ValueOrEnd,
    Value,
    CommaOrEnd,
}

struct JsonScanner {
    chars: Vec<char>,
    offset: usize,
}

impl JsonScanner {
    const MAX_CONTAINER_DEPTH: usize = 9_997;

    fn new(source: &str) -> Self {
        Self {
            chars: source.chars().collect(),
            offset: 0,
        }
    }

    fn scan(mut self) -> Result<Option<JsonSyntaxError>, JsonScanError> {
        self.space();
        if self.offset == self.chars.len() {
            return Ok(Some(self.error("Expecting value", self.offset)));
        }
        let mut frames = Vec::new();
        self.value(&mut frames)?;
        while let Some(frame) = frames.pop() {
            self.space();
            match frame {
                JsonFrame::Object(state) => match state {
                    ObjectState::KeyOrEnd if self.take('}') => {}
                    ObjectState::KeyOrEnd | ObjectState::Key => {
                        if self.peek() != Some('"') {
                            return Ok(Some(self.error(
                                "Expecting property name enclosed in double quotes",
                                self.offset,
                            )));
                        }
                        if let Err(error) = self.string() {
                            return Ok(Some(error));
                        }
                        frames.push(JsonFrame::Object(ObjectState::Colon));
                    }
                    ObjectState::Colon => {
                        if !self.take(':') {
                            return Ok(Some(self.error("Expecting ':' delimiter", self.offset)));
                        }
                        frames.push(JsonFrame::Object(ObjectState::Value));
                    }
                    ObjectState::Value => {
                        frames.push(JsonFrame::Object(ObjectState::CommaOrEnd));
                        self.value(&mut frames)?;
                    }
                    ObjectState::CommaOrEnd if self.take('}') => {}
                    ObjectState::CommaOrEnd if self.take(',') => {
                        frames.push(JsonFrame::Object(ObjectState::Key));
                    }
                    ObjectState::CommaOrEnd => {
                        return Ok(Some(self.error("Expecting ',' delimiter", self.offset)));
                    }
                },
                JsonFrame::Array(state) => match state {
                    ArrayState::ValueOrEnd if self.take(']') => {}
                    ArrayState::ValueOrEnd | ArrayState::Value => {
                        frames.push(JsonFrame::Array(ArrayState::CommaOrEnd));
                        self.value(&mut frames)?;
                    }
                    ArrayState::CommaOrEnd if self.take(']') => {}
                    ArrayState::CommaOrEnd if self.take(',') => {
                        frames.push(JsonFrame::Array(ArrayState::Value));
                    }
                    ArrayState::CommaOrEnd => {
                        return Ok(Some(self.error("Expecting ',' delimiter", self.offset)));
                    }
                },
            }
        }
        self.space();
        Ok((self.offset != self.chars.len()).then(|| self.error("Extra data", self.offset)))
    }

    fn value(&mut self, frames: &mut Vec<JsonFrame>) -> Result<(), JsonScanError> {
        match self.peek() {
            Some('{') => self.open_container(frames, true),
            Some('[') => self.open_container(frames, false),
            Some('"') => self.string().map_err(JsonScanError::Syntax),
            Some('t') => self.literal("true"),
            Some('f') => self.literal("false"),
            Some('n') => self.literal("null"),
            Some('N') => self.literal("NaN"),
            Some('I') => self.literal("Infinity"),
            Some('-') if self.chars.get(self.offset + 1) == Some(&'I') => self.literal("-Infinity"),
            Some('-' | '0'..='9') => self.number(),
            _ => Err(JsonScanError::Syntax(
                self.error("Expecting value", self.offset),
            )),
        }
    }

    fn open_container(
        &mut self,
        frames: &mut Vec<JsonFrame>,
        object: bool,
    ) -> Result<(), JsonScanError> {
        if frames.len() >= Self::MAX_CONTAINER_DEPTH {
            return Err(JsonScanError::Recursion(if object {
                "object"
            } else {
                "array"
            }));
        }
        self.offset += 1;
        frames.push(if object {
            JsonFrame::Object(ObjectState::KeyOrEnd)
        } else {
            JsonFrame::Array(ArrayState::ValueOrEnd)
        });
        Ok(())
    }

    fn string(&mut self) -> Result<(), JsonSyntaxError> {
        let start = self.offset;
        self.offset += 1;
        while let Some(ch) = self.peek() {
            match ch {
                '"' => {
                    self.offset += 1;
                    return Ok(());
                }
                '\\' => {
                    let slash = self.offset;
                    self.offset += 1;
                    match self.peek() {
                        Some('"' | '\\' | '/' | 'b' | 'f' | 'n' | 'r' | 't') => self.offset += 1,
                        Some('u') => {
                            self.offset += 1;
                            for _ in 0..4 {
                                if !self.peek().is_some_and(|c| c.is_ascii_hexdigit()) {
                                    return Err(self.error("Invalid \\uXXXX escape", slash + 1));
                                }
                                self.offset += 1;
                            }
                        }
                        Some(_) => return Err(self.error("Invalid \\escape", slash)),
                        None => return Err(self.error("Unterminated string starting at", start)),
                    }
                }
                ch if ch <= '\u{1f}' => {
                    return Err(self.error("Invalid control character at", self.offset));
                }
                _ => self.offset += 1,
            }
        }
        Err(self.error("Unterminated string starting at", start))
    }

    fn number(&mut self) -> Result<(), JsonScanError> {
        let start = self.offset;
        if self.take('-') && self.offset == self.chars.len() {
            return Err(JsonScanError::Syntax(self.error("Expecting value", start)));
        }
        if self.take('0') {
        } else if self.peek().is_some_and(|c| ('1'..='9').contains(&c)) {
            self.offset += 1;
            while self.peek().is_some_and(|c| c.is_ascii_digit()) {
                self.offset += 1;
            }
        } else {
            return Err(JsonScanError::Syntax(self.error("Expecting value", start)));
        }
        if self.take('.') {
            if !self.peek().is_some_and(|c| c.is_ascii_digit()) {
                self.offset -= 1;
                return Ok(());
            }
            while self.peek().is_some_and(|c| c.is_ascii_digit()) {
                self.offset += 1;
            }
        }
        let exponent = self.offset;
        if self.take('e') || self.take('E') {
            if self.peek().is_some_and(|c| c == '+' || c == '-') {
                self.offset += 1;
            }
            if !self.peek().is_some_and(|c| c.is_ascii_digit()) {
                self.offset = exponent;
                return Ok(());
            }
            while self.peek().is_some_and(|c| c.is_ascii_digit()) {
                self.offset += 1;
            }
        }
        Ok(())
    }

    fn literal(&mut self, literal: &str) -> Result<(), JsonScanError> {
        let start = self.offset;
        for expected in literal.chars() {
            if self.peek() != Some(expected) {
                return Err(JsonScanError::Syntax(self.error("Expecting value", start)));
            }
            self.offset += 1;
        }
        Ok(())
    }

    fn skip_value(&mut self) -> Result<(), JsonScanError> {
        self.space();
        match self.peek() {
            Some('"') => self.string().map_err(JsonScanError::Syntax),
            Some(open @ ('{' | '[')) => {
                let mut closing = vec![if open == '{' { '}' } else { ']' }];
                self.offset += 1;
                while !closing.is_empty() {
                    match self.peek() {
                        Some('"') => self.string().map_err(JsonScanError::Syntax)?,
                        Some(open @ ('{' | '[')) => {
                            closing.push(if open == '{' { '}' } else { ']' });
                            self.offset += 1;
                        }
                        Some(ch) if closing.last() == Some(&ch) => {
                            closing.pop();
                            self.offset += 1;
                        }
                        Some(_) => self.offset += 1,
                        None => {
                            return Err(JsonScanError::Syntax(
                                self.error("Expecting value", self.offset),
                            ));
                        }
                    }
                }
                Ok(())
            }
            Some(_) => {
                while self
                    .peek()
                    .is_some_and(|ch| !matches!(ch, ' ' | '\t' | '\n' | '\r' | ',' | ']' | '}'))
                {
                    self.offset += 1;
                }
                Ok(())
            }
            None => Err(JsonScanError::Syntax(
                self.error("Expecting value", self.offset),
            )),
        }
    }

    fn space(&mut self) {
        while self
            .peek()
            .is_some_and(|c| matches!(c, ' ' | '\t' | '\n' | '\r'))
        {
            self.offset += 1;
        }
    }
    fn peek(&self) -> Option<char> {
        self.chars.get(self.offset).copied()
    }
    fn take(&mut self, ch: char) -> bool {
        if self.peek() == Some(ch) {
            self.offset += 1;
            true
        } else {
            false
        }
    }
    fn error(&self, reason: &'static str, offset: usize) -> JsonSyntaxError {
        JsonSyntaxError { reason, offset }
    }
}

fn config_field_name(raw: &[char]) -> Option<String> {
    let mut decoded = String::new();
    let mut index = 1;
    while index + 1 < raw.len() {
        let ch = raw[index];
        index += 1;
        if ch != '\\' {
            decoded.push(ch);
            continue;
        }
        let escape = *raw.get(index)?;
        index += 1;
        match escape {
            '"' | '\\' | '/' => decoded.push(escape),
            'b' => decoded.push('\u{0008}'),
            'f' => decoded.push('\u{000c}'),
            'n' => decoded.push('\n'),
            'r' => decoded.push('\r'),
            't' => decoded.push('\t'),
            'u' => {
                let high = parse_hex_quad(raw, &mut index)?;
                let scalar = match high {
                    0xd800..=0xdbff => {
                        if raw.get(index) != Some(&'\\') || raw.get(index + 1) != Some(&'u') {
                            return None;
                        }
                        index += 2;
                        let low = parse_hex_quad(raw, &mut index)?;
                        if !(0xdc00..=0xdfff).contains(&low) {
                            return None;
                        }
                        0x10000 + ((u32::from(high) - 0xd800) << 10) + (u32::from(low) - 0xdc00)
                    }
                    0xdc00..=0xdfff => return None,
                    value => u32::from(value),
                };
                decoded.push(char::from_u32(scalar)?);
            }
            _ => return None,
        }
    }
    (index == raw.len() - 1).then_some(decoded)
}

fn parse_hex_quad(raw: &[char], index: &mut usize) -> Option<u16> {
    let mut value = 0_u16;
    for _ in 0..4 {
        value = value.checked_mul(16)? + raw.get(*index)?.to_digit(16)? as u16;
        *index += 1;
    }
    Some(value)
}

struct ProjectedConfig {
    json: String,
    max_errors_is_invalid: bool,
    checks_error_type: Option<&'static str>,
}

struct CheckEntry {
    key: String,
    key_json: String,
    value: usize,
}

enum CheckNodeKind {
    Object(Vec<CheckEntry>),
    Scalar {
        start: usize,
        end: usize,
        value_type: CheckValueType,
    },
}

#[derive(Clone, Copy)]
enum CheckValueType {
    Boolean,
    Other(&'static str),
}

struct CheckNode {
    kind: CheckNodeKind,
    flatten_error: Option<&'static str>,
}

#[derive(Clone, Copy)]
enum CheckObjectState {
    KeyOrEnd,
    Key,
    Colon,
    Value,
    Child,
    CommaOrEnd,
}

struct CheckObjectFrame {
    entries: Vec<CheckEntry>,
    pending_key: Option<(String, String)>,
    state: CheckObjectState,
}

fn project_config_fields(source: &str) -> Option<ProjectedConfig> {
    let mut scanner = JsonScanner::new(source);
    scanner.space();
    if !scanner.take('{') {
        return None;
    }

    let mut fields = [None; 3];
    let mut max_errors_is_invalid = false;
    let mut checks_error_type = None;
    let mut checks_json = None;
    scanner.space();
    while !scanner.take('}') {
        let key_start = scanner.offset;
        scanner.string().ok()?;
        let key_end = scanner.offset;
        scanner.space();
        if !scanner.take(':') {
            return None;
        }
        let field_index = match config_field_name(&scanner.chars[key_start..key_end]).as_deref() {
            Some("max_errors") => Some(0),
            Some("checks") => Some(1),
            Some("per_file_checks") => Some(2),
            _ => None,
        };
        scanner.space();
        let value_start = scanner.offset;
        scanner.skip_value().ok()?;
        if let Some(field_index) = field_index {
            fields[field_index] = Some((value_start, scanner.offset));
            let raw_value = &scanner.chars[value_start..scanner.offset];
            match field_index {
                0 => {
                    max_errors_is_invalid = contains_nonfinite_number(raw_value)
                        || contains_lone_surrogate_escape(raw_value);
                }
                1 => {
                    if contains_nonfinite_number(raw_value)
                        || contains_lone_surrogate_in_json(raw_value)
                    {
                        let normalized = normalize_checks_value(raw_value)?;
                        checks_error_type = normalized.1;
                        checks_json = Some(if checks_error_type.is_some() {
                            "{}".to_owned()
                        } else {
                            normalized.0
                        });
                    } else {
                        checks_error_type = None;
                        checks_json = None;
                    }
                }
                _ => {}
            }
        }
        scanner.space();
        if scanner.take('}') {
            break;
        }
        if !scanner.take(',') {
            return None;
        }
        scanner.space();
    }

    let mut projected = String::from("{");
    for (index, (name, span)) in ["max_errors", "checks", "per_file_checks"]
        .into_iter()
        .zip(fields)
        .enumerate()
    {
        if let Some((start, end)) = span {
            if projected.len() > 1 {
                projected.push(',');
            }
            projected.push('"');
            projected.push_str(name);
            projected.push_str("\":");
            if index == 0 && max_errors_is_invalid {
                projected.push_str("null");
            } else if index == 1 {
                if let Some(checks) = &checks_json {
                    projected.push_str(checks);
                } else {
                    projected.extend(scanner.chars[start..end].iter().copied());
                }
            } else {
                projected.extend(scanner.chars[start..end].iter().copied());
            }
        }
    }
    projected.push('}');
    Some(ProjectedConfig {
        json: projected,
        max_errors_is_invalid,
        checks_error_type,
    })
}

fn normalize_checks_value(raw: &[char]) -> Option<(String, Option<&'static str>)> {
    let mut scanner = JsonScanner {
        chars: raw.to_vec(),
        offset: 0,
    };
    let mut frames = Vec::<CheckObjectFrame>::new();
    let mut nodes = Vec::<CheckNode>::new();
    let mut completed = None;
    let root = loop {
        if let Some(value) = completed.take() {
            if let Some(frame) = frames.last_mut() {
                let (key, key_json) = frame.pending_key.take()?;
                if let Some(entry) = frame.entries.iter_mut().find(|entry| entry.key == key) {
                    entry.value = value;
                    entry.key_json = key_json;
                } else {
                    frame.entries.push(CheckEntry {
                        key,
                        key_json,
                        value,
                    });
                }
                frame.state = CheckObjectState::CommaOrEnd;
                continue;
            }
            break value;
        }

        if frames.is_empty() {
            scanner.space();
            if scanner.take('{') {
                frames.push(CheckObjectFrame {
                    entries: Vec::new(),
                    pending_key: None,
                    state: CheckObjectState::KeyOrEnd,
                });
                continue;
            }
            let start = scanner.offset;
            scanner.skip_value().ok()?;
            let value_type = check_scalar_type(&scanner.chars[start..scanner.offset]);
            let id = nodes.len();
            nodes.push(CheckNode {
                kind: CheckNodeKind::Scalar {
                    start,
                    end: scanner.offset,
                    value_type,
                },
                flatten_error: match value_type {
                    CheckValueType::Boolean => None,
                    CheckValueType::Other(name) => Some(name),
                },
            });
            completed = Some(id);
            continue;
        }

        let frame = frames.last_mut()?;
        scanner.space();
        match frame.state {
            CheckObjectState::KeyOrEnd if scanner.take('}') => {
                let frame = frames.pop()?;
                let flatten_error = frame
                    .entries
                    .iter()
                    .find_map(|entry| nodes[entry.value].flatten_error);
                let id = nodes.len();
                nodes.push(CheckNode {
                    kind: CheckNodeKind::Object(frame.entries),
                    flatten_error,
                });
                completed = Some(id);
            }
            CheckObjectState::KeyOrEnd | CheckObjectState::Key => {
                let key_start = scanner.offset;
                scanner.string().ok()?;
                let key_end = scanner.offset;
                let raw_key = &scanner.chars[key_start..key_end];
                let decoded = config_field_name(raw_key);
                let key = decoded
                    .clone()
                    .unwrap_or_else(|| raw_key.iter().collect::<String>());
                let key_json = decoded
                    .as_ref()
                    .and_then(|key| serde_json::to_string(key).ok())
                    .unwrap_or_else(|| raw_key.iter().collect());
                frame.pending_key = Some((key, key_json));
                frame.state = CheckObjectState::Colon;
            }
            CheckObjectState::Colon => {
                if !scanner.take(':') {
                    return None;
                }
                frame.state = CheckObjectState::Value;
            }
            CheckObjectState::Value => {
                if scanner.take('{') {
                    frame.state = CheckObjectState::Child;
                    frames.push(CheckObjectFrame {
                        entries: Vec::new(),
                        pending_key: None,
                        state: CheckObjectState::KeyOrEnd,
                    });
                } else {
                    let start = scanner.offset;
                    scanner.skip_value().ok()?;
                    let value_type = check_scalar_type(&scanner.chars[start..scanner.offset]);
                    let flatten_error = match value_type {
                        CheckValueType::Boolean => None,
                        CheckValueType::Other(name) => Some(name),
                    };
                    let id = nodes.len();
                    nodes.push(CheckNode {
                        kind: CheckNodeKind::Scalar {
                            start,
                            end: scanner.offset,
                            value_type,
                        },
                        flatten_error,
                    });
                    completed = Some(id);
                }
            }
            CheckObjectState::Child => return None,
            CheckObjectState::CommaOrEnd if scanner.take('}') => {
                let frame = frames.pop()?;
                let flatten_error = frame
                    .entries
                    .iter()
                    .find_map(|entry| nodes[entry.value].flatten_error);
                let id = nodes.len();
                nodes.push(CheckNode {
                    kind: CheckNodeKind::Object(frame.entries),
                    flatten_error,
                });
                completed = Some(id);
            }
            CheckObjectState::CommaOrEnd if scanner.take(',') => {
                frame.state = CheckObjectState::Key;
            }
            CheckObjectState::CommaOrEnd => return None,
        }
    };

    let root_node = &nodes[root];
    let root_error = match root_node.kind {
        CheckNodeKind::Object(_) => root_node.flatten_error,
        CheckNodeKind::Scalar {
            value_type: CheckValueType::Boolean,
            ..
        } => Some("bool"),
        CheckNodeKind::Scalar {
            value_type: CheckValueType::Other(name),
            ..
        } => Some(name),
    };
    let mut json = String::new();
    enum EmitPart {
        Node(usize),
        Text(&'static str),
        Owned(String),
    }
    let mut pending = vec![EmitPart::Node(root)];
    while let Some(part) = pending.pop() {
        match part {
            EmitPart::Text(value) => json.push_str(value),
            EmitPart::Owned(value) => json.push_str(&value),
            EmitPart::Node(id) => match &nodes[id].kind {
                CheckNodeKind::Object(entries) => {
                    json.push('{');
                    pending.push(EmitPart::Text("}"));
                    for (index, entry) in entries.iter().enumerate().rev() {
                        pending.push(EmitPart::Node(entry.value));
                        pending.push(EmitPart::Text(":"));
                        pending.push(EmitPart::Owned(entry.key_json.clone()));
                        if index > 0 {
                            pending.push(EmitPart::Text(","));
                        }
                    }
                }
                CheckNodeKind::Scalar { start, end, .. } => {
                    json.extend(scanner.chars[*start..*end].iter().copied());
                }
            },
        }
    }
    Some((json, root_error))
}

fn check_scalar_type(raw: &[char]) -> CheckValueType {
    let token: String = raw.iter().collect();
    match token.as_str() {
        "true" | "false" => CheckValueType::Boolean,
        "null" => CheckValueType::Other("NoneType"),
        "NaN" | "Infinity" | "-Infinity" => CheckValueType::Other("float"),
        _ if token.starts_with('"') => CheckValueType::Other("str"),
        _ if token.starts_with('[') => CheckValueType::Other("list"),
        _ if token.contains('.') || token.contains('e') || token.contains('E') => {
            CheckValueType::Other("float")
        }
        _ => CheckValueType::Other("int"),
    }
}

fn contains_lone_surrogate_in_json(raw: &[char]) -> bool {
    let mut scanner = JsonScanner {
        chars: raw.to_vec(),
        offset: 0,
    };
    while scanner.offset < scanner.chars.len() {
        if scanner.peek() == Some('"') {
            let start = scanner.offset;
            if scanner.string().is_err() {
                return false;
            }
            if contains_lone_surrogate_escape(&scanner.chars[start..scanner.offset]) {
                return true;
            }
        } else {
            scanner.offset += 1;
        }
    }
    false
}

fn contains_nonfinite_number(raw: &[char]) -> bool {
    let mut scanner = JsonScanner {
        chars: raw.to_vec(),
        offset: 0,
    };
    while scanner.offset < scanner.chars.len() {
        if scanner.peek() == Some('"') {
            if scanner.string().is_err() {
                return false;
            }
            continue;
        }
        let start = scanner.offset;
        while scanner.peek().is_some_and(|ch| {
            !matches!(
                ch,
                ' ' | '\t' | '\n' | '\r' | ',' | ':' | '[' | ']' | '{' | '}'
            )
        }) {
            scanner.offset += 1;
        }
        if scanner.offset == start {
            scanner.offset += 1;
            continue;
        }
        let token: String = scanner.chars[start..scanner.offset].iter().collect();
        if matches!(token.as_str(), "NaN" | "Infinity" | "-Infinity")
            || ((token.contains('.') || token.contains('e') || token.contains('E'))
                && token.parse::<f64>().is_ok_and(|number| !number.is_finite()))
        {
            return true;
        }
    }
    false
}

fn contains_lone_surrogate_escape(raw: &[char]) -> bool {
    let mut index = 1;
    while index + 1 < raw.len() {
        if raw[index] != '\\' {
            index += 1;
            continue;
        }
        index += 1;
        if raw.get(index) != Some(&'u') {
            index += 1;
            continue;
        }
        index += 1;
        let Some(high) = parse_hex_quad(raw, &mut index) else {
            return false;
        };
        match high {
            0xd800..=0xdbff => {
                if raw.get(index) != Some(&'\\') || raw.get(index + 1) != Some(&'u') {
                    return true;
                }
                index += 2;
                let Some(low) = parse_hex_quad(raw, &mut index) else {
                    return false;
                };
                if !(0xdc00..=0xdfff).contains(&low) {
                    return true;
                }
            }
            0xdc00..=0xdfff => return true,
            _ => {}
        }
    }
    false
}

fn python_json_error_message(source: &str, error: JsonSyntaxError) -> String {
    let normalized = source.replace("\r\n", "\n").replace('\r', "\n");
    let prefix: Vec<char> = normalized.chars().take(error.offset).collect();
    let line = prefix.iter().filter(|&&ch| ch == '\n').count() + 1;
    let column = prefix.iter().rev().take_while(|&&ch| ch != '\n').count() + 1;
    format!(
        "{}: line {line} column {column} (char {})",
        error.reason, error.offset
    )
}

fn recursion_error_message(kind: &str) -> String {
    format!("maximum recursion depth exceeded while decoding a JSON {kind} from a unicode string")
}

struct StackSafeValue(Value);

impl std::ops::Deref for StackSafeValue {
    type Target = Value;
    fn deref(&self) -> &Value {
        &self.0
    }
}

impl Drop for StackSafeValue {
    fn drop(&mut self) {
        let mut pending = vec![std::mem::replace(&mut self.0, Value::Null)];
        while let Some(mut value) = pending.pop() {
            match &mut value {
                Value::Array(children) => pending.extend(std::mem::take(children)),
                Value::Object(children) => pending.extend(std::mem::take(children).into_values()),
                _ => {}
            }
        }
    }
}

fn python_utf8_decode_error_message(bytes: &[u8], error: std::str::Utf8Error) -> String {
    let position = error.valid_up_to();
    let length = error.error_len().unwrap_or(bytes.len() - position);
    let end = position + length - 1;
    let reason = if error.error_len().is_none() {
        "unexpected end of data"
    } else if (0xc2..=0xf4).contains(&bytes[position]) {
        "invalid continuation byte"
    } else {
        "invalid start byte"
    };
    let span = if end == position {
        format!("byte 0x{:02x} in position {position}", bytes[position])
    } else {
        format!("bytes in position {position}-{end}")
    };
    format!("'utf-8' codec can't decode {span}: {reason}")
}

#[derive(Debug, Parser)]
#[command(name = "proselint", about = "A linter for prose")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Check {
        #[arg(long)]
        config: Option<PathBuf>,
        #[arg(
            long = "output-format",
            short = 'o',
            value_enum,
            default_value = "full"
        )]
        output_format: OutputFormat,
        #[arg(value_name = "PATH")]
        paths: Vec<PathBuf>,
    },
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum OutputFormat {
    Full,
    Json,
    Compact,
}

fn run() -> Result<ExitCode, String> {
    let cli = Cli::parse();
    let Command::Check {
        config,
        output_format,
        paths,
    } = cli.command;
    let config = match config {
        Some(config) => config,
        None => {
            let cwd = std::env::current_dir()
                .map_err(|error| format!("Unable to determine current directory: {error}"))?;
            let project_config = std::iter::successors(Some(cwd.as_path()), |path| path.parent())
                .map(|directory| directory.join("proselint.json"))
                .find(|candidate| candidate.is_file());
            let user_config = if project_config.is_none() {
                std::env::var_os("XDG_CONFIG_HOME")
                    .filter(|path| !path.is_empty())
                    .map(|path| PathBuf::from(path).join("proselint/config.json"))
                    .or_else(|| {
                        std::env::var_os("HOME")
                            .map(|home| PathBuf::from(home).join(".config/proselint/config.json"))
                    })
                    .filter(|candidate| candidate.is_file())
            } else {
                None
            };
            project_config
                .or(user_config)
                .unwrap_or_else(|| PathBuf::from("proselint.json"))
        }
    };

    if let Ok(metadata) = fs::metadata(&config)
        && metadata.is_dir()
        && let Ok(path) = config.canonicalize()
    {
        let message = format!(
            "[Errno Directories not permitted - found directory %s] {}",
            path.display()
        );
        if matches!(output_format, OutputFormat::Json) {
            println!(
                "{}",
                serde_json::json!({"error":{"code":-31999,"data":"OSError","message":message}})
            );
            return Ok(ExitCode::from(4));
        }
        return Err(format!(
            "Unrecoverable error encountered while running proselint: {message}"
        ));
    }

    let config_bytes = match fs::read(&config) {
        Ok(bytes) => bytes,
        Err(error) => {
            let message = format!(
                "Configuration file '{}' could not be read.",
                config.display()
            );
            if error.kind() == io::ErrorKind::NotFound
                && matches!(output_format, OutputFormat::Json)
            {
                println!(
                    "{}",
                    serde_json::json!({"error":{"code":-31998,"message":message}})
                );
                return Ok(ExitCode::from(4));
            }
            return Err(format!("{message}: {error}"));
        }
    };
    let config_text = match String::from_utf8(config_bytes) {
        Ok(text) => text,
        Err(error) if matches!(output_format, OutputFormat::Json) => {
            let message = python_utf8_decode_error_message(error.as_bytes(), error.utf8_error());
            println!(
                "{}",
                serde_json::json!({"error":{"code":-31999,"message":message,"data":"UnicodeDecodeError"}})
            );
            return Ok(ExitCode::from(4));
        }
        Err(_) => {
            return Err(format!(
                "Configuration file '{}' could not be read.: stream did not contain valid UTF-8",
                config.display()
            ));
        }
    };
    let normalized_config_text = config_text.replace("\r\n", "\n").replace('\r', "\n");
    let scan_result = if config_text.starts_with('\u{feff}') {
        Err(JsonScanError::Syntax(JsonSyntaxError {
            reason: "Unexpected UTF-8 BOM (decode using utf-8-sig)",
            offset: 0,
        }))
    } else {
        JsonScanner::new(&normalized_config_text).scan()
    };
    let syntax_error = match scan_result {
        Ok(error) => error,
        Err(JsonScanError::Syntax(error)) => Some(error),
        Err(JsonScanError::Recursion(kind)) => {
            let message = recursion_error_message(kind);
            if matches!(output_format, OutputFormat::Json) {
                println!(
                    "{}",
                    serde_json::json!({"error":{"code":-31999,"message":message,"data":"RecursionError"}})
                );
                return Ok(ExitCode::from(4));
            }
            return Err(format!(
                "Configuration file '{}' is not valid JSON: {message}",
                config.display()
            ));
        }
    };
    let projected_config = if syntax_error.is_none() {
        project_config_fields(&config_text)
    } else {
        None
    };
    let parse_source = projected_config
        .as_ref()
        .map_or(config_text.as_str(), |config| config.json.as_str());
    let parse_result = (|| {
        let mut deserializer = serde_json::Deserializer::from_str(parse_source);
        deserializer.disable_recursion_limit();
        let value = Value::deserialize(serde_stacker::Deserializer::new(&mut deserializer));
        let value = value?;
        deserializer.end()?;
        Ok::<_, serde_json::Error>(StackSafeValue(value))
    })();
    let config_value = match parse_result {
        Ok(value) => value,
        Err(error) => {
            if matches!(output_format, OutputFormat::Json) {
                let Some(syntax_error) = syntax_error else {
                    return Err(format!(
                        "Configuration file '{}' was rejected by serde_json even though the Python JSON scanner found no syntax error: {error}",
                        config.display()
                    ));
                };
                println!(
                    "{}",
                    serde_json::json!({"error":{"code":-31999,"message":python_json_error_message(&normalized_config_text, syntax_error),"data":"JSONDecodeError"}})
                );
                return Ok(ExitCode::from(4));
            }
            return Err(format!(
                "Configuration file '{}' is not valid JSON: {error}",
                config.display()
            ));
        }
    };
    if !config_value.is_object() {
        return Err(format!(
            "Configuration file '{}' must contain a JSON object",
            config.display()
        ));
    }
    let checks = config_value
        .get("checks")
        .ok_or_else(|| "this capability requires an explicit checks configuration".to_owned())?;
    if projected_config
        .as_ref()
        .is_some_and(|config| config.checks_error_type.is_some())
    {
        let type_name = projected_config
            .as_ref()
            .and_then(|config| config.checks_error_type)
            .unwrap_or("object");
        println!(
            "{}",
            serde_json::json!({"error":{"code":-31999,"data":"AttributeError","message":format!("'{type_name}' object has no attribute 'items'")}})
        );
        return Ok(ExitCode::from(4));
    }
    let max_errors = config_value
        .get("max_errors")
        .and_then(Value::as_u64)
        .and_then(|limit| usize::try_from(limit).ok())
        .unwrap_or(0);

    if matches!(output_format, OutputFormat::Compact) {
        return Err("this capability currently supports JSON output only".to_owned());
    }
    if paths.len() > 1 {
        return Err("this capability currently supports one source at a time".to_owned());
    }

    let (source, content) = if let Some(path) = paths.first() {
        let source = file_uri_or_missing(path)
            .map_err(|error| format!("Could not resolve '{}': {error}", path.display()))?;
        let content = match fs::read_to_string(path) {
            Ok(content) => normalize_file_newlines(&content),
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                let resolved_path = resolved_path(path).unwrap_or_else(|_| path.to_path_buf());
                let message = format!(
                    "[Errno 2] No such file or directory: '{}'",
                    resolved_path.display()
                );
                let mut file_results = serde_json::Map::new();
                file_results.insert(
                    source.clone(),
                    serde_json::json!({"error":{"code":-31998,"message":message}}),
                );
                println!("{}", serde_json::json!({"result":file_results}));
                return Ok(ExitCode::SUCCESS);
            }
            Err(error) => return Err(format!("Could not read '{}': {error}", path.display())),
        };
        (source, content)
    } else {
        let mut input = String::new();
        io::stdin()
            .read_to_string(&mut input)
            .map_err(|error| format!("Could not read stdin: {error}"))?;
        ("<stdin>".to_owned(), input)
    };

    if projected_config
        .as_ref()
        .is_some_and(|config| config.max_errors_is_invalid)
    {
        let mut file_results = serde_json::Map::new();
        file_results.insert(
            source,
            serde_json::json!({"error":{"code":-31999,"data":"ValueError","message":"Stop argument for islice() must be None or an integer: 0 <= x <= sys.maxsize."}}),
        );
        println!("{}", serde_json::json!({"result":file_results}));
        return Ok(ExitCode::SUCCESS);
    }

    let mut selected_checks = supported_checks(checks);
    if let Some(path) = paths.first()
        && let Some(per_file_checks) = config_value
            .get("per_file_checks")
            .and_then(Value::as_object)
        && let Some((_, override_checks)) = per_file_checks
            .iter()
            .find(|(pattern, _)| path_matches_pattern(path, pattern))
    {
        selected_checks = selected_checks.map(|selected| {
            selected
                .into_iter()
                .filter(|check| {
                    effective_check_enabled(checks, override_checks, selected_check_path(*check))
                        != Some(false)
                })
                .collect()
        });
    }
    let selected_checks = selected_checks.ok_or_else(|| {
        "this capability supports disabled checks, annotations.misc, cliches.hell, misc.apologizing, misc.but, misc.whence, misc.suddenly, misc.debased, misc.currency, misc.inferior_superior, misc.false_plurals.kudos, psychology.mental_telepathy, psychology.p_equals_zero, psychology.lie_detector, misc.illogic.coin, misc.illogic.collusion, misc.back_formations, misc.many_a, misc.metadiscourse, misc.pretension, misc.latin, misc.capitalization.seasons, misc.capitalization.terms, misc.capitalization.days, misc.capitalization.months, misc.institution_name, misc.professions, misc.greylist, misc.composition, redundancy.misc.wallace, industrial_language.bureaucratese, industrial_language.jargon, industrial_language.airlinese, hedging, typography.symbols.ellipsis, typography.symbols.copyright, typography.symbols.sentence_spacing, typography.symbols.trademark, typography.symbols.multiplication, dates_times.am_pm.lowercase_periods, dates_times.am_pm.spacing, spelling.ally_ly, spelling.ance_ence, spelling.ely_ly, or terms.eponymous_adjectives only".to_owned()
    })?;

    let mut diagnostics = Vec::new();
    for selected_check in selected_checks {
        let check_diagnostics = match selected_check {
            SelectedCheck::AnnotationsMisc => annotation_diagnostics_in_matcher_order(&content),
            SelectedCheck::ClichesHell => cliches_hell_diagnostics(&content),
            SelectedCheck::MiscApologizing => apologizing_diagnostics(&content),
            SelectedCheck::MiscPhrasalAdjectivesLy => phrasal_adjectives_ly_diagnostics(&content),
            SelectedCheck::MiscBut => but_diagnostics(&content),
            SelectedCheck::MiscWhence => whence_diagnostics(&content),
            SelectedCheck::MiscSuddenly => suddenly_diagnostics(&content),
            SelectedCheck::MiscDebased => debased_diagnostics(&content),
            SelectedCheck::MiscCurrency => currency_diagnostics(&content),
            SelectedCheck::MiscInferiorSuperior => inferior_superior_diagnostics(&content),
            SelectedCheck::DatesTimesAmPmLowercasePeriods => {
                am_pm_lowercase_periods_diagnostics(&content)
            }
            SelectedCheck::DatesTimesAmPmSpacing => am_pm_spacing_diagnostics(&content),
            SelectedCheck::MiscFalsePluralsKudos => false_plurals_kudos_diagnostics(&content),
            SelectedCheck::MiscFalsePluralsMisc => {
                false_plurals_misc_diagnostics_in_matcher_order(&content)
            }
            SelectedCheck::PsychologyMentalTelepathy => mental_telepathy_diagnostics(&content),
            SelectedCheck::PsychologyPEqualsZero => p_equals_zero_diagnostics(&content),
            SelectedCheck::PsychologyLieDetector => lie_detector_diagnostics(&content),
            SelectedCheck::RedundancyMiscWallace => wallace_diagnostics(&content),
            SelectedCheck::MiscIllogicCoin => illogic_coin_diagnostics(&content),
            SelectedCheck::MiscIllogicCollusion => illogic_collusion_diagnostics(&content),
            SelectedCheck::MiscBackFormations => back_formations_diagnostics(&content),
            SelectedCheck::MiscGreylist => greylist_diagnostics(&content),
            SelectedCheck::IndustrialLanguageBureaucratese => bureaucratese_diagnostics(&content),
            SelectedCheck::Hedging => hedging_diagnostics(&content),
            SelectedCheck::IndustrialLanguageJargon => jargon_diagnostics(&content),
            SelectedCheck::IndustrialLanguageAirlinese => airlinese_diagnostics(&content),
            SelectedCheck::MiscManyA => many_a_diagnostics(&content),
            SelectedCheck::MiscMetadiscourse => metadiscourse_diagnostics(&content),
            SelectedCheck::MiscPretension => pretension_diagnostics(&content),
            SelectedCheck::MiscLatin => latin_diagnostics(&content),
            SelectedCheck::MiscCapitalizationSeasons => {
                capitalization_seasons_diagnostics(&content)
            }
            SelectedCheck::MiscCapitalizationTerms => capitalization_terms_diagnostics(&content),
            SelectedCheck::MiscCapitalizationDays => capitalization_days_diagnostics(&content),
            SelectedCheck::MiscCapitalizationMonths => capitalization_months_diagnostics(&content),
            SelectedCheck::MiscInstitutionName => institution_name_diagnostics(&content),
            SelectedCheck::MiscProfessions => professions_diagnostics(&content),
            SelectedCheck::SpellingAllyLy => spelling_ally_ly_diagnostics(&content),
            SelectedCheck::SpellingAnceEnce => spelling_ance_ence_diagnostics(&content),
            SelectedCheck::SpellingElyLy => spelling_ely_ly_diagnostics(&content),
            SelectedCheck::MiscComposition => composition_diagnostics_in_matcher_order(&content),
            SelectedCheck::TermsEponymousAdjectives => eponymous_adjectives_diagnostics(&content),
            SelectedCheck::TypographySymbolsEllipsis => typography_ellipsis_diagnostics(&content),
            SelectedCheck::TypographySymbolsCopyright => typography_copyright_diagnostics(&content),
            SelectedCheck::TypographySymbolsSentenceSpacing => {
                typography_sentence_spacing_diagnostics(&content)
            }
            SelectedCheck::TypographySymbolsTrademark => typography_trademark_diagnostics(&content),
            SelectedCheck::TypographySymbolsMultiplication => {
                typography_multiplication_diagnostics(&content)
            }
        };
        diagnostics.extend(check_diagnostics);
    }
    diagnostics.truncate(max_errors);
    diagnostics.sort_by_key(|diagnostic| {
        (
            diagnostic["pos"][0].as_u64().unwrap_or_default(),
            diagnostic["pos"][1].as_u64().unwrap_or_default(),
        )
    });
    let exit_code = if diagnostics.is_empty() {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    };
    match output_format {
        OutputFormat::Json => println!("{}", json_result(&source, diagnostics)),
        OutputFormat::Full => {
            for diagnostic in diagnostics {
                println!(
                    "{source}:{}:{}: {}: {}",
                    diagnostic["pos"][0].as_u64().unwrap_or_default(),
                    diagnostic["pos"][1].as_u64().unwrap_or_default(),
                    diagnostic["check_path"].as_str().unwrap_or_default(),
                    diagnostic["message"].as_str().unwrap_or_default(),
                );
            }
        }
        OutputFormat::Compact => unreachable!(),
    }
    Ok(exit_code)
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
// Variant order mirrors registration order in the pinned check modules.
enum SelectedCheck {
    AnnotationsMisc,
    ClichesHell,
    DatesTimesAmPmLowercasePeriods,
    DatesTimesAmPmSpacing,
    Hedging,
    IndustrialLanguageAirlinese,
    IndustrialLanguageBureaucratese,
    IndustrialLanguageJargon,
    MiscApologizing,
    MiscBackFormations,
    MiscBut,
    MiscCapitalizationTerms,
    MiscCapitalizationSeasons,
    MiscCapitalizationMonths,
    MiscCapitalizationDays,
    MiscComposition,
    MiscCurrency,
    MiscDebased,
    MiscFalsePluralsMisc,
    MiscFalsePluralsKudos,
    MiscGreylist,
    MiscIllogicCoin,
    MiscIllogicCollusion,
    MiscInferiorSuperior,
    MiscInstitutionName,
    MiscLatin,
    MiscManyA,
    MiscMetadiscourse,
    MiscPhrasalAdjectivesLy,
    MiscPretension,
    MiscProfessions,
    SpellingAllyLy,
    MiscSuddenly,
    MiscWhence,
    PsychologyLieDetector,
    PsychologyPEqualsZero,
    PsychologyMentalTelepathy,
    RedundancyMiscWallace,
    SpellingAnceEnce,
    SpellingElyLy,
    TermsEponymousAdjectives,
    TypographySymbolsEllipsis,
    TypographySymbolsCopyright,
    TypographySymbolsTrademark,
    TypographySymbolsSentenceSpacing,
    TypographySymbolsMultiplication,
}

fn supported_checks(checks: &Value) -> Option<Vec<SelectedCheck>> {
    fn check_for_path(path: &[String]) -> Option<SelectedCheck> {
        let path: Vec<&str> = path.iter().map(String::as_str).collect();
        match path.as_slice() {
            ["annotations", "misc"] => Some(SelectedCheck::AnnotationsMisc),
            ["cliches", "hell"] => Some(SelectedCheck::ClichesHell),
            ["misc", "apologizing"] => Some(SelectedCheck::MiscApologizing),
            ["misc", "phrasal_adjectives", "ly"] => Some(SelectedCheck::MiscPhrasalAdjectivesLy),
            ["misc", "but"] => Some(SelectedCheck::MiscBut),
            ["misc", "whence"] => Some(SelectedCheck::MiscWhence),
            ["misc", "suddenly"] => Some(SelectedCheck::MiscSuddenly),
            ["misc", "debased"] => Some(SelectedCheck::MiscDebased),
            ["misc", "currency"] => Some(SelectedCheck::MiscCurrency),
            ["misc", "inferior_superior"] => Some(SelectedCheck::MiscInferiorSuperior),
            ["dates_times", "am_pm", "lowercase_periods"] => {
                Some(SelectedCheck::DatesTimesAmPmLowercasePeriods)
            }
            ["dates_times", "am_pm", "spacing"] => Some(SelectedCheck::DatesTimesAmPmSpacing),
            ["misc", "false_plurals", "kudos"] => Some(SelectedCheck::MiscFalsePluralsKudos),
            ["misc", "false_plurals", "misc"] => Some(SelectedCheck::MiscFalsePluralsMisc),
            ["psychology", "mental_telepathy"] => Some(SelectedCheck::PsychologyMentalTelepathy),
            ["psychology", "p_equals_zero"] => Some(SelectedCheck::PsychologyPEqualsZero),
            ["psychology", "lie_detector"] => Some(SelectedCheck::PsychologyLieDetector),
            ["redundancy", "misc", "wallace"] => Some(SelectedCheck::RedundancyMiscWallace),
            ["misc", "illogic", "coin"] => Some(SelectedCheck::MiscIllogicCoin),
            ["misc", "illogic", "collusion"] => Some(SelectedCheck::MiscIllogicCollusion),
            ["misc", "back_formations"] => Some(SelectedCheck::MiscBackFormations),
            ["misc", "greylist"] => Some(SelectedCheck::MiscGreylist),
            ["industrial_language", "bureaucratese"] => {
                Some(SelectedCheck::IndustrialLanguageBureaucratese)
            }
            ["hedging"] => Some(SelectedCheck::Hedging),
            ["industrial_language", "jargon"] => Some(SelectedCheck::IndustrialLanguageJargon),
            ["industrial_language", "airlinese"] => {
                Some(SelectedCheck::IndustrialLanguageAirlinese)
            }
            ["misc", "many_a"] => Some(SelectedCheck::MiscManyA),
            ["misc", "metadiscourse"] => Some(SelectedCheck::MiscMetadiscourse),
            ["misc", "pretension"] => Some(SelectedCheck::MiscPretension),
            ["misc", "latin"] => Some(SelectedCheck::MiscLatin),
            ["misc", "capitalization", "seasons"] => Some(SelectedCheck::MiscCapitalizationSeasons),
            ["misc", "capitalization", "terms"] => Some(SelectedCheck::MiscCapitalizationTerms),
            ["misc", "capitalization", "days"] => Some(SelectedCheck::MiscCapitalizationDays),
            ["misc", "capitalization", "months"] => Some(SelectedCheck::MiscCapitalizationMonths),
            ["misc", "institution_name"] => Some(SelectedCheck::MiscInstitutionName),
            ["misc", "professions"] => Some(SelectedCheck::MiscProfessions),
            ["spelling", "ally_ly"] => Some(SelectedCheck::SpellingAllyLy),
            ["spelling", "ance_ence"] => Some(SelectedCheck::SpellingAnceEnce),
            ["spelling", "ely_ly"] => Some(SelectedCheck::SpellingElyLy),
            ["misc", "composition"] => Some(SelectedCheck::MiscComposition),
            ["terms", "eponymous_adjectives"] => Some(SelectedCheck::TermsEponymousAdjectives),
            ["typography", "symbols", "ellipsis"] => Some(SelectedCheck::TypographySymbolsEllipsis),
            ["typography", "symbols", "copyright"] => {
                Some(SelectedCheck::TypographySymbolsCopyright)
            }
            ["typography", "symbols", "sentence_spacing"] => {
                Some(SelectedCheck::TypographySymbolsSentenceSpacing)
            }
            ["typography", "symbols", "trademark"] => {
                Some(SelectedCheck::TypographySymbolsTrademark)
            }
            ["typography", "symbols", "multiplication"] => {
                Some(SelectedCheck::TypographySymbolsMultiplication)
            }
            _ => None,
        }
    }

    if !checks.is_object() {
        return None;
    }
    let mut pending = vec![(checks, Vec::<String>::new())];
    let mut selected = Vec::new();
    while let Some((value, path)) = pending.pop() {
        if let Some(check) = check_for_path(&path) {
            match value {
                Value::Bool(false) => continue,
                Value::Bool(true) => {
                    selected.push(check);
                    continue;
                }
                _ => return None,
            }
        }
        match value {
            Value::Object(entries) => {
                for (key, child) in entries.iter().rev() {
                    let mut child_path = path.clone();
                    child_path.push(key.clone());
                    pending.push((child, child_path));
                }
            }
            Value::Bool(false) => {}
            _ => return None,
        }
    }
    selected.sort_unstable();
    Some(selected)
}

fn main() -> ExitCode {
    match run() {
        Ok(code) => code,
        Err(message) => {
            eprintln!("{message}");
            ExitCode::from(4)
        }
    }
}

fn flatten_checks(value: &Value, prefix: &str, checks: &mut Vec<(String, bool)>) {
    match value {
        Value::Object(entries) => {
            for (key, value) in entries {
                let path = if prefix.is_empty() {
                    key.clone()
                } else {
                    format!("{prefix}.{key}")
                };
                flatten_checks(value, &path, checks);
            }
        }
        Value::Bool(enabled) => checks.push((prefix.to_owned(), *enabled)),
        _ => {}
    }
}

fn effective_check_enabled(global: &Value, per_file: &Value, selected: &str) -> Option<bool> {
    let mut candidates = Vec::new();
    flatten_checks(global, "", &mut candidates);
    let global_len = candidates.len();
    flatten_checks(per_file, "", &mut candidates);
    candidates
        .iter()
        .enumerate()
        .filter(|(_, (path, _))| selected == path || selected.starts_with(&format!("{path}.")))
        .max_by_key(|(index, (path, _))| {
            (path.matches('.').count(), usize::from(*index >= global_len))
        })
        .map(|(_, (_, enabled))| *enabled)
}

fn selected_check_path(check: SelectedCheck) -> &'static str {
    match check {
        SelectedCheck::AnnotationsMisc => "annotations.misc",
        SelectedCheck::ClichesHell => "cliches.hell",
        SelectedCheck::MiscApologizing => "misc.apologizing",
        SelectedCheck::MiscPhrasalAdjectivesLy => "misc.phrasal_adjectives.ly",
        SelectedCheck::MiscBut => "misc.but",
        SelectedCheck::MiscWhence => "misc.whence",
        SelectedCheck::MiscSuddenly => "misc.suddenly",
        SelectedCheck::MiscDebased => "misc.debased",
        SelectedCheck::MiscCurrency => "misc.currency",
        SelectedCheck::MiscInferiorSuperior => "misc.inferior_superior",
        SelectedCheck::DatesTimesAmPmLowercasePeriods => "dates_times.am_pm.lowercase_periods",
        SelectedCheck::DatesTimesAmPmSpacing => "dates_times.am_pm.spacing",
        SelectedCheck::MiscFalsePluralsKudos => "misc.false_plurals.kudos",
        SelectedCheck::MiscFalsePluralsMisc => "misc.false_plurals.misc",
        SelectedCheck::PsychologyMentalTelepathy => "psychology.mental_telepathy",
        SelectedCheck::PsychologyPEqualsZero => "psychology.p_equals_zero",
        SelectedCheck::PsychologyLieDetector => "psychology.lie_detector",
        SelectedCheck::RedundancyMiscWallace => "redundancy.misc.wallace",
        SelectedCheck::MiscIllogicCoin => "misc.illogic.coin",
        SelectedCheck::MiscIllogicCollusion => "misc.illogic.collusion",
        SelectedCheck::MiscBackFormations => "misc.back_formations",
        SelectedCheck::MiscGreylist => "misc.greylist",
        SelectedCheck::IndustrialLanguageBureaucratese => "industrial_language.bureaucratese",
        SelectedCheck::Hedging => "hedging",
        SelectedCheck::IndustrialLanguageJargon => "industrial_language.jargon",
        SelectedCheck::IndustrialLanguageAirlinese => "industrial_language.airlinese",
        SelectedCheck::MiscManyA => "misc.many_a",
        SelectedCheck::MiscMetadiscourse => "misc.metadiscourse",
        SelectedCheck::MiscPretension => "misc.pretension",
        SelectedCheck::MiscLatin => "misc.latin",
        SelectedCheck::MiscCapitalizationSeasons => "misc.capitalization.seasons",
        SelectedCheck::MiscCapitalizationTerms => "misc.capitalization.terms",
        SelectedCheck::MiscCapitalizationDays => "misc.capitalization.days",
        SelectedCheck::MiscCapitalizationMonths => "misc.capitalization.months",
        SelectedCheck::MiscInstitutionName => "misc.institution_name",
        SelectedCheck::MiscProfessions => "misc.professions",
        SelectedCheck::SpellingAllyLy => "spelling.ally_ly",
        SelectedCheck::SpellingAnceEnce => "spelling.ance_ence",
        SelectedCheck::SpellingElyLy => "spelling.ely_ly",
        SelectedCheck::MiscComposition => "misc.composition",
        SelectedCheck::TermsEponymousAdjectives => "terms.eponymous_adjectives",
        SelectedCheck::TypographySymbolsEllipsis => "typography.symbols.ellipsis",
        SelectedCheck::TypographySymbolsCopyright => "typography.symbols.copyright",
        SelectedCheck::TypographySymbolsSentenceSpacing => "typography.symbols.sentence_spacing",
        SelectedCheck::TypographySymbolsTrademark => "typography.symbols.trademark",
        SelectedCheck::TypographySymbolsMultiplication => "typography.symbols.multiplication",
    }
}

fn path_matches_pattern(path: &std::path::Path, pattern: &str) -> bool {
    let path_text = path.to_string_lossy();
    if pattern.contains('/') {
        let path_text = path_text.trim_start_matches('/');
        wildcard_match(path_text, pattern)
            || path_text
                .split('/')
                .collect::<Vec<_>>()
                .windows(pattern.split('/').count())
                .any(|parts| wildcard_match(&parts.join("/"), pattern))
    } else {
        path.file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| wildcard_match(name, pattern))
    }
}

fn wildcard_match(text: &str, pattern: &str) -> bool {
    fn matches(text: &[char], pattern: &[char]) -> bool {
        match pattern.first() {
            None => text.is_empty(),
            Some('*') => (0..=text.len()).any(|offset| {
                (offset == text.len() || text[offset] != '/')
                    && matches(&text[offset..], &pattern[1..])
            }),
            Some('?') => !text.is_empty() && text[0] != '/' && matches(&text[1..], &pattern[1..]),
            Some('[') => {
                let Some(end) = pattern.iter().position(|ch| *ch == ']') else {
                    return !text.is_empty()
                        && text[0] == '['
                        && matches(&text[1..], &pattern[1..]);
                };
                if text.is_empty() || text[0] == '/' || end == 1 {
                    return false;
                }
                let mut class = &pattern[1..end];
                let negated = class.first().is_some_and(|ch| *ch == '!' || *ch == '^');
                if negated {
                    class = &class[1..];
                }
                let contains = class.iter().enumerate().any(|(index, ch)| {
                    if index + 2 < class.len() && class[index + 1] == '-' {
                        *ch <= text[0] && text[0] <= class[index + 2]
                    } else if index > 0 && class[index - 1] == '-' {
                        false
                    } else {
                        *ch == text[0]
                    }
                });
                (contains != negated) && matches(&text[1..], &pattern[end + 1..])
            }
            Some(ch) => !text.is_empty() && text[0] == *ch && matches(&text[1..], &pattern[1..]),
        }
    }
    matches(
        &text.chars().collect::<Vec<_>>(),
        &pattern.chars().collect::<Vec<_>>(),
    )
}

fn file_uri_or_missing(path: &std::path::Path) -> Result<String, io::Error> {
    if let Ok(uri) = file_uri(path) {
        return Ok(uri);
    }
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()?.join(path)
    };
    let resolved = resolved_path(&absolute)?;
    let mut uri = String::from("file://");
    for byte in resolved.to_string_lossy().as_bytes() {
        if byte.is_ascii_alphanumeric() || b"/-._~".contains(byte) {
            uri.push(char::from(*byte));
        } else {
            use std::fmt::Write as _;
            write!(&mut uri, "%{byte:02X}").expect("writing to String cannot fail");
        }
    }
    Ok(uri)
}

fn resolved_path(path: &std::path::Path) -> Result<PathBuf, io::Error> {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()?.join(path)
    };
    let mut normalized = PathBuf::new();
    for component in absolute.components() {
        match component {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                normalized.pop();
            }
            other => normalized.push(other.as_os_str()),
        }
    }
    let mut ancestor = normalized.as_path();
    let mut suffix = Vec::new();
    while !ancestor.exists() {
        let Some(name) = ancestor.file_name() else {
            break;
        };
        suffix.push(name.to_os_string());
        ancestor = ancestor
            .parent()
            .unwrap_or_else(|| std::path::Path::new("/"));
    }
    let mut resolved = ancestor.canonicalize()?;
    for component in suffix.iter().rev() {
        resolved.push(component);
    }
    Ok(resolved)
}
