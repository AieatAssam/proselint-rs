use clap::{ArgAction, CommandFactory, Parser, Subcommand, ValueEnum};
use proselint_rs::{
    airlinese_diagnostics, am_pm_lowercase_periods_diagnostics, am_pm_spacing_diagnostics,
    animal_adjectives_diagnostics, annotation_diagnostics_in_matcher_order,
    apologizing_diagnostics, archaism_diagnostics, back_formations_diagnostics,
    bureaucratese_diagnostics, but_diagnostics, capitalization_days_diagnostics_in_matcher_order,
    capitalization_months_diagnostics_in_matcher_order,
    capitalization_seasons_diagnostics_in_matcher_order,
    capitalization_terms_diagnostics_in_matcher_order, chatspeak_diagnostics,
    cliches_garner_diagnostics, cliches_gnu_diction_diagnostics, cliches_hell_diagnostics,
    cliches_write_good_diagnostics, commercialese_diagnostics,
    composition_diagnostics_in_matcher_order, corporate_speak_diagnostics, currency_diagnostics,
    dates_times_dates_dash_and_from_diagnostics, dates_times_dates_month_of_year_diagnostics,
    dates_times_dates_month_year_comma_diagnostics, debased_diagnostics_in_matcher_order,
    decade_apostrophes_diagnostics, denizen_labels_garner_diagnostics,
    denizen_labels_norris_diagnostics, eponymous_adjectives_diagnostics, exclamation_diagnostics,
    false_plurals_kudos_diagnostics, false_plurals_misc_diagnostics_in_matcher_order, file_uri,
    greylist_diagnostics_in_matcher_order, hedging_diagnostics, hyperbole_diagnostics,
    illogic_coin_diagnostics, illogic_collusion_diagnostics, inferior_superior_diagnostics,
    institution_name_diagnostics, jargon_diagnostics, json_result, latin_diagnostics,
    lexical_illusions_diagnostics, lie_detector_diagnostics, malapropisms_diagnostics,
    many_a_diagnostics_in_matcher_order, mental_telepathy_diagnostics,
    metadiscourse_diagnostics_in_matcher_order, midnight_noon_diagnostics,
    misc_illogic_diagnostics, misc_preferred_forms_diagnostics, misc_tense_present_diagnostics,
    misc_waxed_diagnostics, misplaced_diagnostics, mixed_metaphors_bottleneck_diagnostics,
    mixed_metaphors_misc_diagnostics, mondegreens_diagnostics, narcissism_diagnostics,
    needless_variants_diagnostics, nonwords_diagnostics, nordquist_diagnostics,
    normalize_file_newlines, not_guilty_diagnostics, oxymorons_diagnostics,
    p_equals_zero_diagnostics, phrasal_adjectives_examples_diagnostics,
    phrasal_adjectives_ly_diagnostics, pretension_diagnostics, professions_diagnostics,
    redundancy_misc_after_the_deadline_diagnostics, redundancy_misc_garner_diagnostics,
    redundancy_ras_syndrome_diagnostics, restricted_elementary_diagnostics,
    restricted_top1000_diagnostics, scare_quotes_diagnostics, skunked_terms_diagnostics,
    social_awareness_lgbtq_offensive_diagnostics, social_awareness_lgbtq_terms_diagnostics,
    social_awareness_sexism_diagnostics, social_nword_diagnostics,
    spelling_able_atable_diagnostics, spelling_able_ible_diagnostics, spelling_ally_ly_diagnostics,
    spelling_ance_ence_diagnostics, spelling_athletes_diagnostics,
    spelling_consistency_diagnostics, spelling_ely_ly_diagnostics,
    spelling_em_im_en_in_diagnostics, spelling_er_or_diagnostics, spelling_in_un_diagnostics,
    spelling_misc_diagnostics, spelling_ve_of_diagnostics, suddenly_diagnostics,
    terms_venery_diagnostics, typography_copyright_diagnostics,
    typography_curly_quotes_diagnostics, typography_diacritical_marks_diagnostics,
    typography_ellipsis_diagnostics, typography_multiplication_diagnostics,
    typography_sentence_spacing_diagnostics, typography_spacing_diagnostics,
    typography_trademark_diagnostics, uncomparables_diagnostics, wallace_diagnostics,
    weasel_words_very_diagnostics, whence_diagnostics,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{HashSet, VecDeque};
use std::ffi::OsString;
use std::fs;
use std::io::{self, Read};
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::OnceLock;

mod default_config;
mod unicode_printability;

const CHECK_HELP: &str = r#"usage: proselint [options] <command> check [-h] [--config CONFIG]
                                           [--output-format {full,json,compact}]
                                           [--verbose] [--version] [--demo]
                                           [paths ...]

positional arguments:
  paths                 target paths to lint

options:
  -h, --help            show this help message and exit
  --config CONFIG       path to a configuration file (`.proselintrc.json`)
  --output-format {full,json,compact}, -o {full,json,compact}
                        the format to display results in
  --verbose, -v         enable verbose logging
  --version             display the current version and exit
  --demo                run proselint against the demo file
"#;

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
    per_file_checks_error_type: Option<&'static str>,
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

fn invalid_per_file_checks_error_type(raw: &[char]) -> Option<&'static str> {
    let mut scanner = JsonScanner {
        chars: raw.to_vec(),
        offset: 0,
    };
    scanner.space();
    if !scanner.take('{') {
        let value: String = raw.iter().collect();
        return python_json_root_type(&value);
    }

    let mut entries = Vec::<(String, Vec<char>)>::new();
    scanner.space();
    while !scanner.take('}') {
        let key_start = scanner.offset;
        scanner.string().ok()?;
        let key_end = scanner.offset;
        let key = config_field_name(&scanner.chars[key_start..key_end])
            .unwrap_or_else(|| scanner.chars[key_start..key_end].iter().collect());
        scanner.space();
        if !scanner.take(':') {
            return None;
        }
        scanner.space();
        let value_start = scanner.offset;
        scanner.skip_value().ok()?;
        let value = scanner.chars[value_start..scanner.offset].to_vec();
        if let Some((_, current)) = entries.iter_mut().find(|(name, _)| name == &key) {
            *current = value;
        } else {
            entries.push((key, value));
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

    for (_, value) in entries {
        if let Some(type_name) = normalize_checks_value(&value)?.1 {
            return Some(type_name);
        }
    }
    None
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
    let mut per_file_checks_error_type = None;
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
                        checks_error_type = normalize_checks_value(raw_value)?.1;
                        checks_json = None;
                    }
                }
                2 => {
                    per_file_checks_error_type = invalid_per_file_checks_error_type(raw_value);
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
        per_file_checks_error_type,
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

fn python_json_root_type(source: &str) -> Option<&'static str> {
    let value = source.trim_start_matches([' ', '\t', '\n', '\r']);
    let first = value.chars().next()?;
    Some(match first {
        '{' => return None,
        '[' => "list",
        '"' => "str",
        't' | 'f' => "bool",
        'n' => "NoneType",
        'N' | 'I' => "float",
        '-' | '0'..='9' => {
            let number = value.trim_end_matches([' ', '\t', '\n', '\r']);
            if number == "-Infinity" || number.contains(['.', 'e', 'E']) {
                "float"
            } else {
                "int"
            }
        }
        _ => return None,
    })
}

fn non_object_config_root_error(
    type_name: &str,
    output_format: OutputFormat,
) -> Result<ExitCode, String> {
    let message = format!("'{type_name}' object has no attribute 'get'");
    if matches!(output_format, OutputFormat::Json) {
        print!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "error": {
                    "code": -31999,
                    "data": "AttributeError",
                    "message": message
                }
            }))
            .expect("error JSON is serializable")
        );
        Ok(ExitCode::from(4))
    } else {
        Err(format!(
            "Unrecoverable error encountered while running proselint: {message}"
        ))
    }
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

fn decode_surrogateescape(bytes: &[u8]) -> Vec<u32> {
    let mut result = Vec::new();
    let mut rest = bytes;
    while !rest.is_empty() {
        match std::str::from_utf8(rest) {
            Ok(valid) => {
                result.extend(valid.chars().map(u32::from));
                break;
            }
            Err(error) => {
                let valid = error.valid_up_to();
                result.extend(
                    std::str::from_utf8(&rest[..valid])
                        .expect("valid prefix")
                        .chars()
                        .map(u32::from),
                );
                result.push(0xdc00 + u32::from(rest[valid]));
                rest = &rest[valid + 1..];
            }
        }
    }
    result
}

#[derive(Debug, Parser)]
#[command(
    name = "proselint",
    about = "proselint, a linter for prose.",
    disable_help_subcommand = true
)]
struct Cli {
    #[arg(
        long,
        global = true,
        value_name = "CONFIG",
        allow_hyphen_values = true,
        help = "path to a configuration file (`.proselintrc.json`)"
    )]
    config: Option<PathBuf>,
    #[arg(
        long = "output-format",
        short = 'o',
        global = true,
        value_enum,
        default_value = "full",
        help = "the format to display results in"
    )]
    output_format: OutputFormat,
    #[arg(
        long,
        short = 'v',
        global = true,
        action = ArgAction::SetTrue,
        help = "enable verbose logging"
    )]
    verbose: bool,
    #[arg(
        long,
        global = true,
        action = ArgAction::SetTrue,
        help = "display the current version and exit"
    )]
    version: bool,
    #[command(subcommand)]
    command: Option<Command>,
}

const PRODUCT_VERSION: &str = concat!(
    "proselint-rs ",
    env!("PROSELINT_RS_RELEASE_VERSION"),
    " (tracks Proselint 0.16.0)"
);

#[derive(Debug, Subcommand)]
enum Command {
    #[command(about = "display the current version and exit")]
    Version,
    #[command(
        about = "run proselint against paths",
        override_help = CHECK_HELP
    )]
    Check {
        #[arg(long, help = "run proselint against the demo file")]
        demo: bool,
        #[arg(value_name = "PATH")]
        paths: Vec<PathBuf>,
    },
    #[command(about = "display the loaded configuration and exit")]
    DumpConfig {
        #[arg(long, help = "display the default configuration")]
        default: bool,
    },
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum OutputFormat {
    Full,
    Json,
    Compact,
}

fn top_level_help() -> String {
    let mut command = Cli::command();
    command.build();
    let mut help = format!(
        "usage: {} [options] <command>\n\n{}\n\noptions:\n",
        command.get_name(),
        command
            .get_about()
            .expect("top-level command has a description")
    );
    help_argument_lines(command.get_arguments().collect(), &mut help);
    help.push_str("\ncommands:\n  \n");
    for subcommand in command.get_subcommands() {
        help.push_str(&format!(
            "    {:<19} {}\n",
            subcommand.get_name(),
            subcommand
                .get_about()
                .map(ToString::to_string)
                .unwrap_or_default()
        ));
    }
    help
}

fn command_help(name: &str) -> Option<String> {
    if name == "check" {
        return Some(CHECK_HELP.to_owned());
    }
    let mut root = Cli::command();
    root.build();
    let command = root.find_subcommand(name)?;
    let mut help = String::new();
    let usage_prefix = format!(
        "usage: proselint [options] <command> {}",
        command.get_name()
    );
    let mut usage_arguments = Vec::new();
    let mut arguments: Vec<_> = command.get_arguments().collect();
    arguments.sort_by_key(|argument| match argument.get_long() {
        Some("help") => 0,
        Some("config") => 1,
        Some("output-format") => 2,
        Some("verbose") => 3,
        Some("version") => 4,
        _ => 5,
    });
    for argument in arguments {
        let token = if matches!(argument.get_action(), ArgAction::Help) {
            argument.get_short().map(|short| format!("-{short}"))
        } else if let Some(long) = argument.get_long() {
            let takes_value = !matches!(
                argument.get_action(),
                ArgAction::SetTrue | ArgAction::SetFalse | ArgAction::Count | ArgAction::Help
            );
            let values = argument.get_possible_values();
            let value = if !takes_value {
                None
            } else if values.is_empty() {
                argument
                    .get_value_names()
                    .and_then(|names| names.first())
                    .map(ToString::to_string)
            } else {
                Some(format!(
                    "{{{}}}",
                    values
                        .iter()
                        .map(|value| value.get_name())
                        .collect::<Vec<_>>()
                        .join(",")
                ))
            };
            Some(match value {
                Some(value) => format!("--{long} {value}"),
                None => format!("--{long}"),
            })
        } else {
            None
        };
        if let Some(token) = token {
            usage_arguments.push(format!("[{token}]"));
        }
    }
    let mut line = usage_prefix.clone();
    for token in usage_arguments {
        if line.len() + 1 + token.len() > 80 {
            help.push_str(&line);
            help.push('\n');
            line = format!("{}{}", " ".repeat(usage_prefix.len() + 1), token);
        } else {
            line.push(' ');
            line.push_str(&token);
        }
    }
    help.push_str(&line);
    help.push_str("\n\noptions:\n");
    help_argument_lines(command.get_arguments().collect(), &mut help);
    Some(help)
}

fn help_argument_lines(mut arguments: Vec<&clap::Arg>, help: &mut String) {
    arguments.sort_by_key(|argument| match argument.get_long() {
        Some("help") => 0,
        Some("config") => 1,
        Some("output-format") => 2,
        Some("verbose") => 3,
        Some("version") => 4,
        _ => 5,
    });
    for argument in arguments {
        let mut names = String::new();
        let possible_values = if matches!(
            argument.get_action(),
            ArgAction::SetTrue | ArgAction::SetFalse | ArgAction::Count | ArgAction::Help
        ) {
            Vec::new()
        } else {
            argument.get_possible_values()
        };
        let takes_value = !matches!(
            argument.get_action(),
            ArgAction::SetTrue | ArgAction::SetFalse | ArgAction::Count | ArgAction::Help
        );
        let value_name = if !takes_value {
            None
        } else if !possible_values.is_empty() {
            Some(format!(
                "{{{}}}",
                possible_values
                    .iter()
                    .map(|value| value.get_name())
                    .collect::<Vec<_>>()
                    .join(",")
            ))
        } else {
            argument
                .get_value_names()
                .and_then(|values| values.first())
                .map(ToString::to_string)
        };
        if argument.get_long() == Some("help")
            && let Some(short) = argument.get_short()
        {
            names.push('-');
            names.push(short);
            names.push_str(", ");
        }
        if let Some(long) = argument.get_long() {
            names.push_str("--");
            names.push_str(long);
        }
        if let Some(value_name) = &value_name {
            names.push(' ');
            names.push_str(value_name);
        }
        if let Some(short) = argument
            .get_short()
            .filter(|_| argument.get_long() != Some("help"))
        {
            if !names.is_empty() {
                names.push_str(", ");
            }
            names.push('-');
            names.push(short);
            if let Some(value_name) = &value_name {
                names.push(' ');
                names.push_str(value_name);
            }
        }
        let description = if argument.get_long() == Some("help") {
            "show this help message and exit".to_owned()
        } else {
            argument
                .get_help()
                .map(ToString::to_string)
                .unwrap_or_default()
        };
        if names.len() <= 22 {
            help.push_str(&format!("  {names:<22}{description}\n"));
        } else {
            help.push_str(&format!("  {names}\n{}{}\n", " ".repeat(24), description));
        }
    }
}

fn user_config_path() -> Option<PathBuf> {
    std::env::var_os("XDG_CONFIG_HOME")
        .filter(|path| !path.is_empty())
        .map(|path| PathBuf::from(path).join("proselint/config.json"))
        .or_else(|| {
            std::env::var_os("HOME")
                .filter(|home| !home.is_empty())
                .map(PathBuf::from)
                .or_else(|| {
                    #[allow(deprecated)]
                    std::env::home_dir()
                })
                .map(|home| home.join(".config/proselint/config.json"))
        })
        .filter(|candidate| candidate.is_file())
}

fn config_directory_error(path: &std::path::Path) -> Option<String> {
    if !fs::metadata(path).ok()?.is_dir() {
        return None;
    }
    Some(format!(
        "[Errno Directories not permitted - found directory %s] {}",
        path.canonicalize().ok()?.display()
    ))
}

fn dump_config(
    config: Option<PathBuf>,
    default: bool,
    version: bool,
    output_format: OutputFormat,
) -> Result<ExitCode, String> {
    let config_path = match config {
        Some(path) => Some(path),
        None => {
            let cwd = std::env::current_dir()
                .map_err(|error| format!("Unable to determine current directory: {error}"))?;
            let project = std::iter::successors(Some(cwd.as_path()), |path| path.parent())
                .map(|directory| directory.join("proselint.json"))
                .find(|candidate| candidate.is_file());
            if project.is_some() {
                project
            } else {
                user_config_path()
            }
        }
    };

    let mut checks_error_type = None;
    let mut per_file_checks_error_type = None;
    let loaded = match config_path {
        Some(path) => {
            if let Some(message) = config_directory_error(&path) {
                if matches!(output_format, OutputFormat::Json) {
                    print!(
                        "{}",
                        serde_json::to_string_pretty(&serde_json::json!({
                            "error": {
                                "code": -31999,
                                "data": "OSError",
                                "message": message
                            }
                        }))
                        .expect("directory error JSON is serializable")
                    );
                    return Ok(ExitCode::from(4));
                }
                return Err(format!(
                    "Unrecoverable error encountered while running proselint: {message}"
                ));
            }
            let source = match fs::read_to_string(&path) {
                Ok(source) => source,
                Err(error) if error.kind() == io::ErrorKind::NotFound => {
                    let message =
                        format!("Configuration file '{}' could not be read.", path.display());
                    if matches!(output_format, OutputFormat::Json) {
                        print!(
                            "{}",
                            serde_json::to_string_pretty(&serde_json::json!({
                                "error": {"code": -31998, "message": message}
                            }))
                            .expect("error JSON is serializable")
                        );
                        return Ok(ExitCode::from(4));
                    }
                    return Err(format!(
                        "Unrecoverable error encountered while running proselint: {message}"
                    ));
                }
                Err(error) => {
                    return Err(format!(
                        "Configuration file '{}' could not be read: {error}",
                        path.display()
                    ));
                }
            };
            let normalized = source.replace("\r\n", "\n").replace('\r', "\n");
            let scan_result = if source.starts_with('\u{feff}') {
                Err(JsonScanError::Syntax(JsonSyntaxError {
                    reason: "Unexpected UTF-8 BOM (decode using utf-8-sig)",
                    offset: 0,
                }))
            } else {
                JsonScanner::new(&normalized).scan()
            };
            match scan_result {
                Ok(Some(error)) | Err(JsonScanError::Syntax(error)) => {
                    return Err(format!(
                        "Unrecoverable error encountered while running proselint: {}",
                        python_json_error_message(&normalized, error)
                    ));
                }
                Err(JsonScanError::Recursion(kind)) => {
                    return Err(format!(
                        "Unrecoverable error encountered while running proselint: {}",
                        recursion_error_message(kind)
                    ));
                }
                Ok(None) => {}
            }
            if let Some(type_name) = python_json_root_type(&normalized) {
                return non_object_config_root_error(type_name, output_format);
            }
            let loaded = serde_json::from_str::<Value>(&source).map_err(|error| {
                format!(
                    "Configuration file '{}' is not valid JSON: {error}",
                    path.display()
                )
            })?;
            let projected_config = project_config_fields(&normalized);
            checks_error_type = projected_config
                .as_ref()
                .and_then(|config| config.checks_error_type);
            per_file_checks_error_type =
                projected_config.and_then(|config| config.per_file_checks_error_type);
            Some(loaded)
        }
        None => None,
    };

    if let Some(type_name) = checks_error_type.or(per_file_checks_error_type) {
        let message = format!("'{type_name}' object has no attribute 'items'");
        if matches!(output_format, OutputFormat::Json) {
            println!(
                "{}",
                serde_json::json!({"error":{"code":-31999,"data":"AttributeError","message":message}})
            );
            return Ok(ExitCode::from(4));
        }
        return Err(format!(
            "Unrecoverable error encountered while running proselint: {message}"
        ));
    }

    if version {
        println!("Proselint 0.16.0");
        return Ok(ExitCode::SUCCESS);
    }

    if default || loaded.is_none() {
        print!("{}", default_config::DEFAULT_CONFIG_JSON);
    } else if let Some(value) = loaded {
        print!("{}", python_json_dump(&normalized_config(&value)?)?);
    }
    Ok(ExitCode::SUCCESS)
}

fn normalized_config(value: &Value) -> Result<Value, String> {
    let Some(root) = value.as_object() else {
        return Err("Configuration must contain a JSON object".to_owned());
    };
    let mut checks = serde_json::Map::new();
    if let Some(value) = root.get("checks") {
        flatten_dump_checks(value, "", &mut checks)?;
    }
    let mut per_file_checks = serde_json::Map::new();
    if let Some(entries) = root.get("per_file_checks").and_then(Value::as_object) {
        for (name, value) in entries {
            let mut overrides = serde_json::Map::new();
            flatten_dump_checks(value, "", &mut overrides)?;
            let mut merged = checks.clone();
            merged.extend(overrides);
            per_file_checks.insert(name.clone(), Value::Object(merged));
        }
    }
    let mut result = serde_json::Map::new();
    result.insert(
        "max_errors".to_owned(),
        root.get("max_errors").cloned().unwrap_or(Value::from(0)),
    );
    result.insert("checks".to_owned(), Value::Object(checks));
    result.insert("per_file_checks".to_owned(), Value::Object(per_file_checks));
    Ok(Value::Object(result))
}

fn flatten_dump_checks(
    value: &Value,
    prefix: &str,
    checks: &mut serde_json::Map<String, Value>,
) -> Result<(), String> {
    match value {
        Value::Object(entries) => {
            for (key, value) in entries {
                let path = if prefix.is_empty() {
                    key.clone()
                } else {
                    format!("{prefix}.{key}")
                };
                flatten_dump_checks(value, &path, checks)?;
            }
        }
        Value::Bool(_) => {
            checks.insert(prefix.to_owned(), value.clone());
        }
        _ => return Err("Check configuration values must be objects or booleans".to_owned()),
    }
    Ok(())
}

fn python_json_dump(value: &Value) -> Result<String, String> {
    fn sorted(value: Value) -> Value {
        match value {
            Value::Array(values) => Value::Array(values.into_iter().map(sorted).collect()),
            Value::Object(values) => {
                let mut entries: Vec<_> = values.into_iter().collect();
                entries.sort_by(|left, right| left.0.cmp(&right.0));
                let mut result = serde_json::Map::new();
                for (key, value) in entries {
                    result.insert(key, sorted(value));
                }
                Value::Object(result)
            }
            value => value,
        }
    }

    fn escape_non_ascii_json(source: &str) -> String {
        let mut result = String::with_capacity(source.len());
        let (mut in_string, mut escaped) = (false, false);
        for character in source.chars() {
            if in_string && !escaped && character != '"' && !character.is_ascii() {
                let mut units = [0u16; 2];
                for unit in character.encode_utf16(&mut units).iter() {
                    use std::fmt::Write as _;
                    write!(result, "\\u{unit:04x}").expect("writing to a String cannot fail");
                }
                continue;
            }
            result.push(character);
            if in_string && !escaped && character == '"' {
                in_string = false;
            } else if in_string && !escaped && character == '\\' {
                escaped = true;
            } else if escaped {
                escaped = false;
            } else if character == '"' {
                in_string = true;
            }
        }
        result
    }

    let mut bytes = Vec::new();
    let formatter = serde_json::ser::PrettyFormatter::with_indent(b"    ");
    let mut serializer = serde_json::Serializer::with_formatter(&mut bytes, formatter);
    sorted(value.clone())
        .serialize(&mut serializer)
        .map_err(|error| format!("Could not serialize configuration: {error}"))?;
    let text = String::from_utf8(bytes)
        .map_err(|error| format!("Serialized configuration was not UTF-8: {error}"))?;
    Ok(format!("{}\n", escape_non_ascii_json(&text)))
}

fn argparse_negative_number(value: &str) -> bool {
    static NEGATIVE_NUMBER: OnceLock<regex::Regex> = OnceLock::new();
    NEGATIVE_NUMBER
        .get_or_init(|| regex::Regex::new(r"^-[\d]+$|^-[\d]*\.[\d]+$").unwrap())
        .is_match(value)
}

fn missing_option_value_before_option(args: &[std::ffi::OsString]) -> Option<ExitCode> {
    let mut subcommand = None;
    let mut index = 1;
    while index < args.len() {
        let Some(token) = args[index].to_str() else {
            index += 1;
            continue;
        };
        if token == "--" {
            break;
        }
        if matches!(token, "--config" | "--output-format" | "-o") {
            if args.get(index + 1).is_none_or(|value| {
                value.to_str().is_some_and(|value| {
                    value.starts_with('-') && value != "-" && !argparse_negative_number(value)
                })
            }) {
                let argument = if token == "--config" {
                    "--config"
                } else {
                    "--output-format/-o"
                };
                let help = subcommand
                    .and_then(command_help)
                    .unwrap_or_else(|| "usage: proselint [options] <command>".to_owned());
                let usage = help
                    .split_once("\n\n")
                    .map_or(help.as_str(), |(usage, _)| usage);
                let prefix = subcommand.map_or_else(
                    || "proselint".to_owned(),
                    |command| format!("proselint [options] <command> {command}"),
                );
                eprint!("{usage}\n{prefix}: error: argument {argument}: expected one argument\n");
                return Some(ExitCode::from(2));
            }
            index = (index + 2).min(args.len());
            continue;
        }
        if token.starts_with("--config=") || token.starts_with("--output-format=") {
            index += 1;
            continue;
        }
        if subcommand.is_none() && matches!(token, "check" | "version" | "dump-config") {
            subcommand = Some(token);
        }
        index += 1;
    }
    None
}

fn invalid_cli_output_format(args: &[std::ffi::OsString]) -> Option<ExitCode> {
    let mut subcommand = None;
    let mut index = 1;
    while index < args.len() {
        let Some(token) = args[index].to_str() else {
            index += 1;
            continue;
        };
        if token == "--" || token == "-h" || token == "--help" {
            break;
        }
        if token == "--config" {
            index = (index + 2).min(args.len());
            continue;
        }
        if token.starts_with("--config=") {
            index += 1;
            continue;
        }
        if subcommand.is_none() && matches!(token, "check" | "version" | "dump-config") {
            subcommand = Some(token);
            index += 1;
            continue;
        }

        let (format_value, consumes_next) =
            if let Some(attached) = token.strip_prefix("--output-format=") {
                (Some(attached), false)
            } else if token == "--output-format" || token == "-o" {
                (args.get(index + 1).and_then(|value| value.to_str()), true)
            } else if let Some(attached) = token.strip_prefix("-o") {
                (Some(attached.strip_prefix('=').unwrap_or(attached)), false)
            } else {
                (None, false)
            };
        if let Some(value) = format_value
            && (!value.starts_with('-') || value == "-" || argparse_negative_number(value))
            && OutputFormat::from_str(value, false).is_err()
        {
            let help = subcommand
                .and_then(command_help)
                .unwrap_or_else(|| "usage: proselint [options] <command>".to_owned());
            let usage = help
                .split_once("\n\n")
                .map_or(help.as_str(), |(usage, _)| usage);
            let prefix = subcommand.map_or_else(
                || "proselint".to_owned(),
                |command| format!("proselint [options] <command> {command}"),
            );
            eprint!(
                "{usage}\n{prefix}: error: argument --output-format/-o: invalid OutputFormat value: {}\n",
                python_string_repr(value)
            );
            return Some(ExitCode::from(2));
        }
        if consumes_next && index + 1 < args.len() {
            index += 1;
        }
        index += 1;
    }
    None
}

fn python_string_repr(value: &str) -> String {
    let quote = if value.contains('\'') && !value.contains('"') {
        '"'
    } else {
        '\''
    };
    let mut rendered = String::with_capacity(value.len() + 2);
    rendered.push(quote);
    for character in value.chars() {
        match character {
            '\\' => rendered.push_str("\\\\"),
            '\n' => rendered.push_str("\\n"),
            '\r' => rendered.push_str("\\r"),
            '\t' => rendered.push_str("\\t"),
            value if value == quote => {
                rendered.push('\\');
                rendered.push(value);
            }
            value if !unicode_printability::is_printable(value) => {
                let codepoint = value as u32;
                if codepoint <= 0xff {
                    rendered.push_str(&format!("\\x{codepoint:02x}"));
                } else if codepoint <= 0xffff {
                    rendered.push_str(&format!("\\u{codepoint:04x}"));
                } else {
                    rendered.push_str(&format!("\\U{codepoint:08x}"));
                }
            }
            value => rendered.push(value),
        }
    }
    rendered.push(quote);
    rendered
}

#[cfg(test)]
mod python_repr_tests {
    use super::python_string_repr;

    #[test]
    fn escapes_python_nonprintable_unicode_and_preserves_quote_rules() {
        assert_eq!(python_string_repr("'\""), "'\\'\"'");
        assert_eq!(python_string_repr("\n\r\t\u{7f}"), "'\\n\\r\\t\\x7f'");
        assert_eq!(
            python_string_repr("\u{2028}\u{2029}\u{200b}\u{00a0}\u{1c89}"),
            "'\\u2028\\u2029\\u200b\\xa0\\u1c89'"
        );
        assert_eq!(python_string_repr("a b"), "'a b'");
    }
}

fn reset_root_options_before_subcommand(args: &mut Vec<std::ffi::OsString>) {
    let mut index = 1;
    let mut root_options = Vec::new();
    let mut command_index = None;
    let mut invalid_root_output_format = false;
    while index < args.len() {
        let Some(token) = args[index].to_str() else {
            index += 1;
            continue;
        };
        if token == "--" {
            break;
        }
        if token == "--config" {
            if args.get(index + 1).is_some_and(|value| {
                value.to_str().is_none_or(|value| {
                    !value.starts_with('-') || value == "-" || argparse_negative_number(value)
                })
            }) {
                root_options.extend([index, index + 1]);
            }
            index = (index + 2).min(args.len());
            continue;
        }
        if token == "--output-format" || token == "-o" {
            if let Some(value) = args.get(index + 1).and_then(|value| value.to_str()) {
                if OutputFormat::from_str(value, false).is_ok() {
                    root_options.extend([index, index + 1]);
                } else {
                    invalid_root_output_format = true;
                }
                index += 2;
            } else {
                invalid_root_output_format = true;
                index += 1;
            }
            continue;
        }
        if token.starts_with("--config=") {
            root_options.push(index);
            index += 1;
            continue;
        }
        if let Some(value) = token
            .strip_prefix("--output-format=")
            .or_else(|| token.strip_prefix("-o").filter(|_| token != "-o"))
        {
            let value = value.strip_prefix('=').unwrap_or(value);
            if OutputFormat::from_str(value, false).is_ok() {
                root_options.push(index);
            } else {
                invalid_root_output_format = true;
            }
            index += 1;
            continue;
        }
        if matches!(token, "check" | "version" | "dump-config") {
            command_index = Some(index);
            break;
        }
        if token == "--version" {
            root_options.push(index);
        }
        index += 1;
    }
    if command_index.is_some() && !invalid_root_output_format {
        for index in root_options.into_iter().rev() {
            args.remove(index);
        }
    }
}

fn abbreviate_long_options(args: &mut [std::ffi::OsString]) -> Option<ExitCode> {
    let mut subcommand = None;
    let mut index = 1;
    while index < args.len() {
        let Some(token) = args[index].to_str().map(str::to_owned) else {
            index += 1;
            continue;
        };
        if token == "--" {
            break;
        }
        if matches!(token.as_str(), "--config" | "--output-format") {
            index = (index + 2).min(args.len());
            continue;
        }
        // shortcut: this mirrors CLI declarations, derive from Clap if options grow.
        let options: &[&str] = match subcommand {
            Some("check") => &[
                "--help",
                "--config",
                "--output-format",
                "--verbose",
                "--version",
                "--demo",
            ],
            Some("dump-config") => &[
                "--help",
                "--config",
                "--output-format",
                "--verbose",
                "--version",
                "--default",
            ],
            Some("version") => &[
                "--help",
                "--config",
                "--output-format",
                "--verbose",
                "--version",
            ],
            _ => &[
                "--help",
                "--config",
                "--output-format",
                "--verbose",
                "--version",
            ],
        };
        if token.starts_with("--") {
            let (option, attached) = token
                .split_once('=')
                .map_or((token.as_str(), false), |(name, _)| (name, true));
            let matches = options
                .iter()
                .copied()
                .filter(|candidate| candidate.starts_with(option))
                .collect::<Vec<_>>();
            if matches.len() > 1 {
                let help = subcommand
                    .and_then(command_help)
                    .unwrap_or_else(|| "usage: proselint [options] <command>".to_owned());
                let usage = help
                    .split_once("\n\n")
                    .map_or(help.as_str(), |(usage, _)| usage);
                let prefix = subcommand.map_or_else(
                    || "proselint".to_owned(),
                    |command| format!("proselint [options] <command> {command}"),
                );
                let option_token = if attached { token.as_str() } else { option };
                eprintln!(
                    "{usage}\n{prefix}: error: ambiguous option: {option_token} could match {}",
                    matches.join(", ")
                );
                return Some(ExitCode::from(2));
            }
            if let [matched] = matches.as_slice() {
                if attached && !matches!(*matched, "--config" | "--output-format") {
                    let help = subcommand
                        .and_then(command_help)
                        .unwrap_or_else(|| "usage: proselint [options] <command>".to_owned());
                    let usage = help
                        .split_once("\n\n")
                        .map_or(help.as_str(), |(usage, _)| usage);
                    let prefix = subcommand.map_or_else(
                        || "proselint".to_owned(),
                        |command| format!("proselint [options] <command> {command}"),
                    );
                    let argument = match *matched {
                        "--help" => "-h/--help",
                        "--verbose" => "--verbose/-v",
                        argument => argument,
                    };
                    let value = token.split_once('=').map_or("", |(_, value)| value);
                    eprintln!(
                        "{usage}\n{prefix}: error: argument {argument}: ignored explicit argument {}",
                        python_string_repr(value)
                    );
                    return Some(ExitCode::from(2));
                }
                args[index] = if attached {
                    format!("{matched}={}", token.split_once('=').unwrap().1).into()
                } else {
                    (*matched).into()
                };
                if !attached && matches!(*matched, "--config" | "--output-format") {
                    index = (index + 2).min(args.len());
                    continue;
                }
            }
        }
        if subcommand.is_none() && matches!(token.as_str(), "check" | "version" | "dump-config") {
            subcommand = Some(match token.as_str() {
                "check" => "check",
                "version" => "version",
                _ => "dump-config",
            });
        }
        index += 1;
    }
    None
}

fn unrecognized_cli_arguments(args: &[std::ffi::OsString]) -> Option<ExitCode> {
    let mut unknown = Vec::new();
    let mut subcommand = None;
    let mut index = 1;
    while index < args.len() {
        let Some(token) = args[index].to_str() else {
            index += 1;
            continue;
        };
        if token == "--" || token == "-h" || token == "--help" {
            break;
        }
        if matches!(token, "--config" | "--output-format" | "-o") {
            index = (index + 2).min(args.len());
            continue;
        }
        if token.starts_with("--config=")
            || token.starts_with("--output-format=")
            || (token.starts_with("-o") && token != "-o")
            || matches!(token, "--verbose" | "-v" | "--version")
            || (token == "--demo" && subcommand == Some("check"))
            || (token == "--default" && subcommand == Some("dump-config"))
            || (!token.starts_with('-') && index > 1)
        {
            if subcommand.is_none() {
                subcommand = match token {
                    "check" | "version" | "dump-config" => Some(token),
                    _ => None,
                };
            }
            index += 1;
            continue;
        }
        if token.starts_with('-') {
            unknown.push(token.to_owned());
        }
        index += 1;
    }
    if unknown.is_empty() {
        return None;
    }
    let parsed = Cli::try_parse_from(args).err()?;
    if parsed.kind() != clap::error::ErrorKind::UnknownArgument {
        return None;
    }
    eprintln!(
        "usage: proselint [options] <command>\nproselint: error: unrecognized arguments: {}",
        unknown.join(" ")
    );
    Some(ExitCode::from(2))
}

fn run() -> Result<ExitCode, String> {
    let mut args = std::env::args_os().collect::<Vec<_>>();
    let standalone_version_flag = args.len() == 2 && args[1] == "--version";
    if standalone_version_flag {
        args[1] = "version".into();
    }
    if let Some(exit_code) = abbreviate_long_options(&mut args) {
        return Ok(exit_code);
    }
    reset_root_options_before_subcommand(&mut args);
    if args.len() == 2 && (args[1] == "--help" || args[1] == "-h") {
        print!("{}", top_level_help());
        return Ok(ExitCode::SUCCESS);
    }
    if let Some(exit_code) = missing_option_value_before_option(&args) {
        return Ok(exit_code);
    }
    if let Some(exit_code) = invalid_cli_output_format(&args) {
        return Ok(exit_code);
    }
    if let Some(exit_code) = unrecognized_cli_arguments(&args) {
        return Ok(exit_code);
    }
    if args.len() == 4 && args[1] == "--config" && args[2] == "check" && args[3] == "--help" {
        print!("{}", top_level_help());
        return Ok(ExitCode::SUCCESS);
    }
    if args.len() == 3
        && (args[2] == "--help" || args[2] == "-h")
        && let Some(name) = args[1].to_str()
        && matches!(name, "check" | "version" | "dump-config")
    {
        print!("{}", command_help(name).expect("known command has help"));
        return Ok(ExitCode::SUCCESS);
    }
    let parsed = Cli::parse_from(args);
    let config = parsed.config;
    let output_format = parsed.output_format;
    let version_flag = parsed.version;
    let _verbose = parsed.verbose;
    let (paths, demo, show_version, product_version, no_command) = match parsed.command {
        Some(Command::Version) => (Vec::new(), false, true, standalone_version_flag, false),
        Some(Command::Check { paths, demo }) => (paths, demo, version_flag, false, false),
        Some(Command::DumpConfig { default }) => {
            return dump_config(config, default, version_flag, output_format);
        }
        None => (Vec::new(), false, version_flag, version_flag, !version_flag),
    };
    let (config, config_is_default, user_config_discovered) = match config {
        Some(config) => (config, false, false),
        None => {
            let cwd = std::env::current_dir()
                .map_err(|error| format!("Unable to determine current directory: {error}"))?;
            let project_config = std::iter::successors(Some(cwd.as_path()), |path| path.parent())
                .map(|directory| directory.join("proselint.json"))
                .find(|candidate| candidate.is_file());
            match project_config {
                Some(config) => (config, false, false),
                None => match user_config_path() {
                    Some(config) => (config, false, true),
                    None => (PathBuf::from("proselint.json"), true, false),
                },
            }
        }
    };

    if let Some(message) = config_directory_error(&config) {
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

    let config_text = if config_is_default {
        default_config::DEFAULT_CONFIG_JSON.to_owned()
    } else {
        let config_bytes = match fs::read(&config) {
            Ok(bytes) => bytes,
            Err(error) => {
                let message = format!(
                    "Configuration file '{}' could not be read.",
                    config.display()
                );
                if error.kind() == io::ErrorKind::NotFound {
                    if matches!(output_format, OutputFormat::Json) {
                        println!(
                            "{}",
                            serde_json::json!({"error":{"code":-31998,"message":message}})
                        );
                        return Ok(ExitCode::from(4));
                    }
                    return Err(format!(
                        "Unrecoverable error encountered while running proselint: {message}"
                    ));
                }
                if user_config_discovered && error.raw_os_error() == Some(13) {
                    let permission_message =
                        format!("[Errno 13] Permission denied: '{}'", config.display());
                    if matches!(output_format, OutputFormat::Json) {
                        println!(
                            "{}",
                            serde_json::json!({"error":{"code":-31998,"message":permission_message}})
                        );
                        return Ok(ExitCode::from(4));
                    }
                    return Err(format!(
                        "Unrecoverable error encountered while running proselint: {permission_message}"
                    ));
                }
                return Err(format!("{message}: {error}"));
            }
        };
        match String::from_utf8(config_bytes) {
            Ok(text) => text,
            Err(error) if matches!(output_format, OutputFormat::Json) => {
                let message =
                    python_utf8_decode_error_message(error.as_bytes(), error.utf8_error());
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
    if syntax_error.is_none()
        && let Some(type_name) = python_json_root_type(&normalized_config_text)
    {
        return non_object_config_root_error(type_name, output_format);
    }
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
    if show_version {
        println!(
            "{}",
            if product_version {
                PRODUCT_VERSION
            } else {
                "Proselint 0.16.0"
            }
        );
        return Ok(ExitCode::SUCCESS);
    }
    if no_command {
        print!("{}", top_level_help());
        return Ok(ExitCode::from(2));
    }
    let empty_checks = Value::Object(serde_json::Map::new());
    let checks = config_value.get("checks").unwrap_or(&empty_checks);
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
    if let Some(type_name) = projected_config
        .as_ref()
        .and_then(|config| config.per_file_checks_error_type)
    {
        let message = format!("'{type_name}' object has no attribute 'items'");
        if matches!(output_format, OutputFormat::Json) {
            println!(
                "{}",
                serde_json::json!({"error":{"code":-31999,"data":"AttributeError","message":message}})
            );
            return Ok(ExitCode::from(4));
        }
        return Err(format!(
            "Unrecoverable error encountered while running proselint: {message}"
        ));
    }
    let mut max_errors_is_invalid = projected_config
        .as_ref()
        .is_some_and(|config| config.max_errors_is_invalid);
    let max_errors = match config_value.get("max_errors") {
        None => Some(0),
        Some(Value::Null) => None,
        Some(Value::Bool(value)) => Some(usize::from(*value)),
        Some(Value::Number(value)) => value
            .as_u64()
            .filter(|limit| *limit <= isize::MAX as u64)
            .and_then(|limit| usize::try_from(limit).ok())
            .or_else(|| {
                max_errors_is_invalid = true;
                Some(0)
            }),
        Some(_) => {
            max_errors_is_invalid = true;
            Some(0)
        }
    };

    let global_selected = if config_is_default {
        Some(default_supported_checks(checks))
    } else {
        supported_checks(checks)
    };
    let mut file_results = serde_json::Map::new();
    let mut has_findings = false;
    let process_source = |path: Option<&PathBuf>,
                          source: &str,
                          full_source: &str,
                          compact_source: &str,
                          content: &str,
                          codepoints: Option<&[u32]>|
     -> Result<(Option<Value>, bool), String> {
        if max_errors_is_invalid {
            return Ok((
                Some(
                    serde_json::json!({"error":{"code":-31999,"data":"ValueError","message":"Stop argument for islice() must be None or an integer: 0 <= x <= sys.maxsize."}}),
                ),
                false,
            ));
        }
        let mut per_file_override = None;
        if !demo
            && let Some(path) = path
            && let Some(per_file_checks) = config_value
                .get("per_file_checks")
                .and_then(Value::as_object)
            && let Some((_, override_checks)) = per_file_checks
                .iter()
                .find(|(pattern, _)| path_matches_pattern(path, pattern))
        {
            per_file_override = Some(override_checks);
        }
        let override_selected = per_file_override.map(supported_checks);
        let selected_checks = match (global_selected.clone(), override_selected) {
            (Some(global), Some(Some(overlay))) => {
                let mut all = global;
                all.extend(overlay);
                all.sort_unstable();
                all.dedup();
                let mut flattened = Vec::new();
                flatten_checks(checks, "", false, &mut flattened);
                if let Some(overlay) = per_file_override {
                    flatten_checks(overlay, "", true, &mut flattened);
                }
                Some(
                    all.into_iter()
                        .filter(|check| {
                            let path = selected_check_path(*check);
                            flattened
                                .iter()
                                .enumerate()
                                .filter(|(_, (candidate, _, _))| {
                                    path == candidate || path.starts_with(&format!("{candidate}."))
                                })
                                .max_by_key(|(index, (candidate, _, per_file))| {
                                    (candidate.matches('.').count(), *per_file, *index)
                                })
                                .is_some_and(|(_, (_, enabled, _))| *enabled)
                        })
                        .collect(),
                )
            }
            (Some(global), None) => Some(global),
            (None, _) | (_, Some(None)) => None,
        };
        let selected_checks = selected_checks.ok_or_else(|| {
            "this capability supports disabled checks, annotations.misc, archaism, cliches.hell, cliches.misc.garner, cliches.misc.write_good, cliches.misc.gnu_diction, malapropisms, misc.apologizing, misc.phrasal_adjectives.examples, misc.preferred_forms, misc.but, misc.whence, misc.waxed, misc.illogic, misc.illogic.coin, misc.illogic.collusion, misc.tense_present, mixed_metaphors.bottleneck, mixed_metaphors.misc, misc.suddenly, misc.debased, misc.currency, misc.inferior_superior, misc.false_plurals.kudos, psychology.mental_telepathy, psychology.p_equals_zero, psychology.lie_detector, misc.back_formations, misc.many_a, misc.metadiscourse, misc.narcissism, misc.not_guilty, misc.pretension, misc.latin, misc.capitalization.seasons, misc.capitalization.terms, misc.capitalization.days, misc.capitalization.months, misc.institution_name, misc.professions, misc.greylist, misc.composition, misc.scare_quotes, mondegreens, needless_variants, nonwords, oxymorons, redundancy.misc.wallace, redundancy.misc.garner, redundancy.misc.nordquist, redundancy.misc.after_the_deadline, redundancy.ras_syndrome, restricted.elementary, restricted.top1000, industrial_language.bureaucratese, industrial_language.chatspeak, industrial_language.commercialese, industrial_language.corporate_speak, industrial_language.jargon, industrial_language.airlinese, hedging, typography.symbols.ellipsis, typography.symbols.copyright, typography.symbols.sentence_spacing, typography.symbols.trademark, typography.symbols.multiplication, typography.symbols.curly_quotes, uncomparables, typography.punctuation.misplaced, typography.punctuation.exclamation, typography.punctuation.hyperbole, typography.punctuation.spacing, typography.diacritical_marks, dates_times.am_pm.lowercase_periods, dates_times.am_pm.spacing, dates_times.am_pm.midnight_noon, dates_times.dates.decade_apostrophes, dates_times.dates.month_of_year, spelling.able_atable, spelling.able_ible, spelling.ally_ly, spelling.ance_ence, spelling.athletes, spelling.consistency, spelling.ely_ly, spelling.em_im_en_in, spelling.er_or, spelling.in_un, spelling.misc, spelling.ve_of, skunked_terms, social_awareness.nword, social_awareness.lgbtq.offensive, social_awareness.lgbtq.terms, social_awareness.sexism, weasel_words.very, terms.animal_adjectives, terms.denizen_labels.garner, terms.denizen_labels.norris, terms.venery, or terms.eponymous_adjectives only".to_owned()
        })?;

        let mut diagnostics = Vec::new();
        let mut consumed = 0usize;
        for selected_check in selected_checks {
            if max_errors.is_some_and(|limit| consumed >= limit) {
                break;
            }
            if selected_check == SelectedCheck::MiscApologizing
                && let Some(codepoints) = codepoints
            {
                let position = codepoints
                    .iter()
                    .position(|&cp| (0xdc80..=0xdcff).contains(&cp));
                if let Some(position) = position {
                    let mut end = position + 1;
                    while end < codepoints.len() && (0xdc80..=0xdcff).contains(&codepoints[end]) {
                        end += 1;
                    }
                    let message = if end == position + 1 {
                        format!(
                            "'utf-8' codec can't encode character '\\udc{:02x}' in position {}: surrogates not allowed",
                            codepoints[position] - 0xdc00,
                            position + 1
                        )
                    } else {
                        format!(
                            "'utf-8' codec can't encode characters in position {}-{}: surrogates not allowed",
                            position + 1,
                            end
                        )
                    };
                    return Ok((
                        Some(
                            serde_json::json!({"error":{"code":-31999,"data":"UnicodeEncodeError","message":message}}),
                        ),
                        false,
                    ));
                }
            }
            let mut check_diagnostics = match selected_check {
                SelectedCheck::AnnotationsMisc => annotation_diagnostics_in_matcher_order(content),
                SelectedCheck::Archaism => archaism_diagnostics(content),
                SelectedCheck::ClichesHell => cliches_hell_diagnostics(content),
                SelectedCheck::ClichesMiscGarner => cliches_garner_diagnostics(content),
                SelectedCheck::ClichesMiscGnuDiction => cliches_gnu_diction_diagnostics(content),
                SelectedCheck::ClichesMiscWriteGood => cliches_write_good_diagnostics(content),
                SelectedCheck::Malapropisms => malapropisms_diagnostics(content),
                SelectedCheck::SkunkedTerms => skunked_terms_diagnostics(content),
                SelectedCheck::SocialAwarenessNword => social_nword_diagnostics(content),
                SelectedCheck::SocialAwarenessLgbtqOffensive => {
                    social_awareness_lgbtq_offensive_diagnostics(content)
                }
                SelectedCheck::SocialAwarenessLgbtqTerms => {
                    social_awareness_lgbtq_terms_diagnostics(content)
                }
                SelectedCheck::SocialAwarenessSexism => {
                    social_awareness_sexism_diagnostics(content)
                }
                SelectedCheck::TypographySymbolsCurlyQuotes => {
                    typography_curly_quotes_diagnostics(content)
                }
                SelectedCheck::Uncomparables => uncomparables_diagnostics(content),
                SelectedCheck::WeaselWordsVery => weasel_words_very_diagnostics(content),
                SelectedCheck::MiscApologizing => apologizing_diagnostics(content),
                SelectedCheck::MiscPhrasalAdjectivesLy => {
                    phrasal_adjectives_ly_diagnostics(content)
                }
                SelectedCheck::MiscPhrasalAdjectivesExamples => {
                    phrasal_adjectives_examples_diagnostics(content)
                }
                SelectedCheck::MiscPreferredForms => misc_preferred_forms_diagnostics(content),
                SelectedCheck::MiscBut => but_diagnostics(content),
                SelectedCheck::MiscWhence => whence_diagnostics(content),
                SelectedCheck::MiscWaxed => misc_waxed_diagnostics(content),
                SelectedCheck::MiscIllogic => misc_illogic_diagnostics(content),
                SelectedCheck::LexicalIllusions => codepoints.map_or_else(
                    || lexical_illusions_diagnostics(content),
                    proselint_rs::rules::lexical_illusions::diagnostics_codepoints,
                ),
                SelectedCheck::MiscTensePresent => misc_tense_present_diagnostics(content),
                SelectedCheck::MixedMetaphorsBottleneck => {
                    mixed_metaphors_bottleneck_diagnostics(content)
                }
                SelectedCheck::MixedMetaphorsMisc => mixed_metaphors_misc_diagnostics(content),
                SelectedCheck::MiscSuddenly => suddenly_diagnostics(content),
                SelectedCheck::MiscDebased => debased_diagnostics_in_matcher_order(content),
                SelectedCheck::MiscCurrency => currency_diagnostics(content),
                SelectedCheck::MiscInferiorSuperior => inferior_superior_diagnostics(content),
                SelectedCheck::DatesTimesAmPmLowercasePeriods => {
                    am_pm_lowercase_periods_diagnostics(content)
                }
                SelectedCheck::DatesTimesAmPmSpacing => am_pm_spacing_diagnostics(content),
                SelectedCheck::DatesTimesAmPmMidnightNoon => midnight_noon_diagnostics(content),
                SelectedCheck::DatesTimesDatesDecadeApostrophes => {
                    decade_apostrophes_diagnostics(content)
                }
                SelectedCheck::DatesTimesDatesDashAndFrom => {
                    dates_times_dates_dash_and_from_diagnostics(content)
                }
                SelectedCheck::DatesTimesDatesMonthYearComma => {
                    dates_times_dates_month_year_comma_diagnostics(content)
                }
                SelectedCheck::DatesTimesDatesMonthOfYear => {
                    dates_times_dates_month_of_year_diagnostics(content)
                }
                SelectedCheck::MiscScareQuotes => scare_quotes_diagnostics(content),
                SelectedCheck::TypographyPunctuationMisplaced => misplaced_diagnostics(content),
                SelectedCheck::TypographyPunctuationExclamation => exclamation_diagnostics(content),
                SelectedCheck::TypographyPunctuationHyperbole => hyperbole_diagnostics(content),
                SelectedCheck::TypographyPunctuationSpacing => {
                    typography_spacing_diagnostics(content)
                }
                SelectedCheck::TypographyDiacriticalMarks => {
                    typography_diacritical_marks_diagnostics(content)
                }
                SelectedCheck::MiscFalsePluralsKudos => false_plurals_kudos_diagnostics(content),
                SelectedCheck::MiscFalsePluralsMisc => {
                    false_plurals_misc_diagnostics_in_matcher_order(content)
                }
                SelectedCheck::PsychologyMentalTelepathy => mental_telepathy_diagnostics(content),
                SelectedCheck::PsychologyPEqualsZero => p_equals_zero_diagnostics(content),
                SelectedCheck::PsychologyLieDetector => lie_detector_diagnostics(content),
                SelectedCheck::RedundancyMiscWallace => wallace_diagnostics(content),
                SelectedCheck::RedundancyMiscGarner => redundancy_misc_garner_diagnostics(content),
                SelectedCheck::RedundancyMiscNordquist => nordquist_diagnostics(content),
                SelectedCheck::RedundancyMiscAfterTheDeadline => {
                    redundancy_misc_after_the_deadline_diagnostics(content)
                }
                SelectedCheck::RedundancyRasSyndrome => {
                    redundancy_ras_syndrome_diagnostics(content)
                }
                SelectedCheck::RestrictedElementary => restricted_elementary_diagnostics(content),
                SelectedCheck::RestrictedTop1000 => restricted_top1000_diagnostics(content),
                SelectedCheck::MiscIllogicCoin => illogic_coin_diagnostics(content),
                SelectedCheck::MiscIllogicCollusion => illogic_collusion_diagnostics(content),
                SelectedCheck::MiscBackFormations => back_formations_diagnostics(content),
                SelectedCheck::MiscGreylist => greylist_diagnostics_in_matcher_order(content),
                SelectedCheck::IndustrialLanguageBureaucratese => {
                    bureaucratese_diagnostics(content)
                }
                SelectedCheck::IndustrialLanguageChatspeak => chatspeak_diagnostics(content),
                SelectedCheck::IndustrialLanguageCorporateSpeak => {
                    corporate_speak_diagnostics(content)
                }
                SelectedCheck::IndustrialLanguageCommercialese => {
                    commercialese_diagnostics(content)
                }
                SelectedCheck::Hedging => hedging_diagnostics(content),
                SelectedCheck::IndustrialLanguageJargon => jargon_diagnostics(content),
                SelectedCheck::IndustrialLanguageAirlinese => airlinese_diagnostics(content),
                SelectedCheck::MiscManyA => many_a_diagnostics_in_matcher_order(content),
                SelectedCheck::MiscMetadiscourse => {
                    metadiscourse_diagnostics_in_matcher_order(content)
                }
                SelectedCheck::MiscNarcissism => narcissism_diagnostics(content),
                SelectedCheck::MiscNotGuilty => not_guilty_diagnostics(content),
                SelectedCheck::MiscPretension => pretension_diagnostics(content),
                SelectedCheck::MiscLatin => latin_diagnostics(content),
                SelectedCheck::MiscCapitalizationSeasons => {
                    capitalization_seasons_diagnostics_in_matcher_order(content)
                }
                SelectedCheck::MiscCapitalizationTerms => {
                    capitalization_terms_diagnostics_in_matcher_order(content)
                }
                SelectedCheck::MiscCapitalizationDays => {
                    capitalization_days_diagnostics_in_matcher_order(content)
                }
                SelectedCheck::MiscCapitalizationMonths => {
                    capitalization_months_diagnostics_in_matcher_order(content)
                }
                SelectedCheck::MiscInstitutionName => institution_name_diagnostics(content),
                SelectedCheck::MiscProfessions => professions_diagnostics(content),
                SelectedCheck::SpellingAllyLy => spelling_ally_ly_diagnostics(content),
                SelectedCheck::SpellingAbleAtable => spelling_able_atable_diagnostics(content),
                SelectedCheck::SpellingAbleIble => spelling_able_ible_diagnostics(content),
                SelectedCheck::SpellingAnceEnce => spelling_ance_ence_diagnostics(content),
                SelectedCheck::SpellingElyLy => spelling_ely_ly_diagnostics(content),
                SelectedCheck::SpellingErOr => spelling_er_or_diagnostics(content),
                SelectedCheck::SpellingEmImEnIn => spelling_em_im_en_in_diagnostics(content),
                SelectedCheck::SpellingAthletes => spelling_athletes_diagnostics(content),
                SelectedCheck::SpellingConsistency => spelling_consistency_diagnostics(content),
                SelectedCheck::SpellingInUn => spelling_in_un_diagnostics(content),
                SelectedCheck::SpellingMisc => spelling_misc_diagnostics(content),
                SelectedCheck::SpellingVeOf => spelling_ve_of_diagnostics(content),
                SelectedCheck::Mondegreens => mondegreens_diagnostics(content),
                SelectedCheck::NeedlessVariants => needless_variants_diagnostics(content),
                SelectedCheck::Nonwords => nonwords_diagnostics(content),
                SelectedCheck::Oxymorons => oxymorons_diagnostics(content),
                SelectedCheck::MiscComposition => composition_diagnostics_in_matcher_order(content),
                SelectedCheck::TermsEponymousAdjectives => {
                    eponymous_adjectives_diagnostics(content)
                }
                SelectedCheck::TermsVenery => terms_venery_diagnostics(content),
                SelectedCheck::TermsAnimalAdjectives => animal_adjectives_diagnostics(content),
                SelectedCheck::TermsDenizenLabelsGarner => {
                    denizen_labels_garner_diagnostics(content)
                }
                SelectedCheck::TermsDenizenLabelsNorris => {
                    denizen_labels_norris_diagnostics(content)
                }
                SelectedCheck::TypographySymbolsEllipsis => {
                    typography_ellipsis_diagnostics(content)
                }
                SelectedCheck::TypographySymbolsCopyright => {
                    typography_copyright_diagnostics(content)
                }
                SelectedCheck::TypographySymbolsSentenceSpacing => {
                    typography_sentence_spacing_diagnostics(content)
                }
                SelectedCheck::TypographySymbolsTrademark => {
                    typography_trademark_diagnostics(content)
                }
                SelectedCheck::TypographySymbolsMultiplication => {
                    typography_multiplication_diagnostics(content)
                }
            };
            if let Some(limit) = max_errors {
                check_diagnostics.truncate(limit.saturating_sub(consumed));
            }
            consumed += check_diagnostics.len();
            diagnostics.extend(check_diagnostics);
        }
        diagnostics.sort_by_key(|diagnostic| {
            (
                diagnostic["pos"][0].as_u64().unwrap_or_default(),
                diagnostic["pos"][1].as_u64().unwrap_or_default(),
            )
        });
        let has_findings = !diagnostics.is_empty();
        let result = match output_format {
            OutputFormat::Json => Some(json_result(source, diagnostics)["result"][source].clone()),
            OutputFormat::Full => {
                for diagnostic in diagnostics {
                    println!(
                        "{full_source}:{}:{}: {}: {}",
                        diagnostic["pos"][0].as_u64().unwrap_or_default(),
                        diagnostic["pos"][1].as_u64().unwrap_or_default(),
                        diagnostic["check_path"].as_str().unwrap_or_default(),
                        diagnostic["message"].as_str().unwrap_or_default(),
                    );
                }
                None
            }
            OutputFormat::Compact => {
                for diagnostic in diagnostics {
                    println!(
                        "{compact_source}:{}:{}: {}: {}",
                        diagnostic["pos"][0].as_u64().unwrap_or_default(),
                        diagnostic["pos"][1].as_u64().unwrap_or_default(),
                        diagnostic["check_path"].as_str().unwrap_or_default(),
                        diagnostic["message"].as_str().unwrap_or_default()
                    );
                }
                None
            }
        };
        Ok((result, has_findings))
    };

    if demo {
        let (result, findings) = process_source(
            None,
            "demo.md",
            "demo.md",
            "demo.md",
            include_str!("../data/demo.md"),
            None,
        )?;
        if let Some(result) = result {
            file_results.insert("demo.md".to_owned(), result);
        }
        has_findings |= findings;
    } else if paths.is_empty() {
        let mut bytes = Vec::new();
        io::stdin()
            .read_to_end(&mut bytes)
            .map_err(|error| format!("Could not read stdin: {error}"))?;
        let codepoints = decode_surrogateescape(&bytes);
        let input: String = codepoints
            .iter()
            .filter_map(|&cp| char::from_u32(cp))
            .collect();
        let (result, findings) = process_source(
            None,
            "<stdin>",
            "<stdin>",
            "<stdin>",
            &input,
            Some(&codepoints),
        )?;
        if let Some(result) = result {
            file_results.insert("<stdin>".to_owned(), result);
        }
        has_findings |= findings;
    } else {
        let expanded_paths = expand_input_paths(&paths);
        for path in &expanded_paths {
            let source = file_uri_or_missing(path)
                .map_err(|error| format!("Could not resolve '{}': {error}", path.display()))?;
            let resolved = resolved_path(path).unwrap_or_else(|_| path.to_path_buf());
            let full = resolved.display().to_string();
            let compact = resolved
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("<stdin>")
                .to_owned();
            let content = match fs::read_to_string(path) {
                Ok(content) => normalize_file_newlines(&content),
                Err(error) if error.kind() == io::ErrorKind::NotFound => {
                    let message = format!("[Errno 2] No such file or directory: '{}'", full);
                    if matches!(output_format, OutputFormat::Json) || max_errors_is_invalid {
                        file_results.insert(
                            source.clone(),
                            serde_json::json!({"error":{"code":-31998,"message":message}}),
                        );
                    } else {
                        let label = match output_format {
                            OutputFormat::Full => &full,
                            OutputFormat::Compact => &compact,
                            OutputFormat::Json => unreachable!(),
                        };
                        eprintln!("Error encountered while linting {label}: {message}");
                    }
                    continue;
                }
                Err(error) if error.kind() == io::ErrorKind::InvalidData => {
                    let bytes = fs::read(path)
                        .map_err(|error| format!("Could not read '{}': {error}", path.display()))?;
                    let utf8_error =
                        std::str::from_utf8(&bytes).expect_err("invalid UTF-8 was just detected");
                    let message = python_utf8_decode_error_message(&bytes, utf8_error);
                    match output_format {
                        OutputFormat::Full => {
                            eprintln!("Error encountered while linting {full}: {message}");
                        }
                        OutputFormat::Compact => {
                            eprintln!("Error encountered while linting {compact}: {message}");
                        }
                        OutputFormat::Json => {
                            file_results.insert(
                                source.clone(),
                                serde_json::json!({"error":{"code":-31999,"data":"UnicodeDecodeError","message":message}}),
                            );
                        }
                    }
                    continue;
                }
                Err(error) => return Err(format!("Could not read '{}': {error}", path.display())),
            };
            let (result, findings) =
                process_source(Some(path), &source, &full, &compact, &content, None)?;
            if let Some(result) = result {
                file_results.insert(source, result);
            }
            has_findings |= findings;
        }
    }
    if matches!(output_format, OutputFormat::Json) || max_errors_is_invalid {
        println!("{}", serde_json::json!({"result": file_results}));
    }
    Ok(if has_findings {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    })
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
// Variant order mirrors registration order in the pinned check modules.
enum SelectedCheck {
    AnnotationsMisc,
    Archaism,
    ClichesHell,
    ClichesMiscGarner,
    ClichesMiscWriteGood,
    ClichesMiscGnuDiction,
    DatesTimesAmPmLowercasePeriods,
    DatesTimesAmPmSpacing,
    DatesTimesAmPmMidnightNoon,
    DatesTimesDatesDecadeApostrophes,
    DatesTimesDatesDashAndFrom,
    DatesTimesDatesMonthYearComma,
    DatesTimesDatesMonthOfYear,
    Hedging,
    IndustrialLanguageAirlinese,
    IndustrialLanguageBureaucratese,
    IndustrialLanguageChatspeak,
    IndustrialLanguageCommercialese,
    IndustrialLanguageCorporateSpeak,
    IndustrialLanguageJargon,
    LexicalIllusions,
    Malapropisms,
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
    MiscIllogic,
    MiscIllogicCoin,
    MiscIllogicCollusion,
    MiscInferiorSuperior,
    MiscInstitutionName,
    MiscLatin,
    MiscManyA,
    MiscMetadiscourse,
    MiscNarcissism,
    MiscNotGuilty,
    MiscPhrasalAdjectivesLy,
    MiscPhrasalAdjectivesExamples,
    MiscPreferredForms,
    MiscPretension,
    MiscProfessions,
    MiscScareQuotes,
    MiscSuddenly,
    MiscTensePresent,
    MiscWaxed,
    MiscWhence,
    MixedMetaphorsBottleneck,
    MixedMetaphorsMisc,
    Mondegreens,
    NeedlessVariants,
    Nonwords,
    Oxymorons,
    PsychologyLieDetector,
    PsychologyPEqualsZero,
    PsychologyMentalTelepathy,
    RedundancyMiscWallace,
    RedundancyMiscGarner,
    RedundancyMiscNordquist,
    RedundancyMiscAfterTheDeadline,
    RedundancyRasSyndrome,
    RestrictedElementary,
    RestrictedTop1000,
    SkunkedTerms,
    SocialAwarenessLgbtqTerms,
    SocialAwarenessLgbtqOffensive,
    SocialAwarenessNword,
    SocialAwarenessSexism,
    SpellingAbleAtable,
    SpellingAbleIble,
    SpellingAllyLy,
    SpellingAnceEnce,
    SpellingAthletes,
    SpellingConsistency,
    SpellingElyLy,
    SpellingEmImEnIn,
    SpellingErOr,
    SpellingInUn,
    SpellingMisc,
    SpellingVeOf,
    TermsAnimalAdjectives,
    TermsDenizenLabelsGarner,
    TermsDenizenLabelsNorris,
    TermsEponymousAdjectives,
    TermsVenery,
    TypographyDiacriticalMarks,
    TypographyPunctuationMisplaced,
    TypographyPunctuationExclamation,
    TypographyPunctuationHyperbole,
    TypographyPunctuationSpacing,
    TypographySymbolsEllipsis,
    TypographySymbolsCopyright,
    TypographySymbolsTrademark,
    TypographySymbolsSentenceSpacing,
    TypographySymbolsMultiplication,
    TypographySymbolsCurlyQuotes,
    Uncomparables,
    WeaselWordsVery,
}

impl SelectedCheck {
    fn all_implemented() -> Vec<Self> {
        use SelectedCheck::*;
        vec![
            AnnotationsMisc,
            Archaism,
            ClichesHell,
            ClichesMiscGarner,
            ClichesMiscWriteGood,
            ClichesMiscGnuDiction,
            DatesTimesAmPmLowercasePeriods,
            DatesTimesAmPmSpacing,
            DatesTimesAmPmMidnightNoon,
            DatesTimesDatesDecadeApostrophes,
            DatesTimesDatesDashAndFrom,
            DatesTimesDatesMonthYearComma,
            DatesTimesDatesMonthOfYear,
            Hedging,
            IndustrialLanguageAirlinese,
            IndustrialLanguageBureaucratese,
            IndustrialLanguageChatspeak,
            IndustrialLanguageCommercialese,
            IndustrialLanguageCorporateSpeak,
            IndustrialLanguageJargon,
            LexicalIllusions,
            Malapropisms,
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
            MiscIllogic,
            MiscIllogicCoin,
            MiscIllogicCollusion,
            MiscInferiorSuperior,
            MiscInstitutionName,
            MiscLatin,
            MiscManyA,
            MiscMetadiscourse,
            MiscNarcissism,
            MiscNotGuilty,
            MiscPhrasalAdjectivesLy,
            MiscPhrasalAdjectivesExamples,
            MiscPreferredForms,
            MiscPretension,
            MiscProfessions,
            MiscScareQuotes,
            MiscSuddenly,
            MiscTensePresent,
            MiscWaxed,
            MiscWhence,
            MixedMetaphorsBottleneck,
            MixedMetaphorsMisc,
            Mondegreens,
            NeedlessVariants,
            Nonwords,
            Oxymorons,
            PsychologyLieDetector,
            PsychologyPEqualsZero,
            PsychologyMentalTelepathy,
            RedundancyMiscWallace,
            RedundancyMiscGarner,
            RedundancyMiscNordquist,
            RedundancyMiscAfterTheDeadline,
            RedundancyRasSyndrome,
            RestrictedElementary,
            RestrictedTop1000,
            SkunkedTerms,
            SocialAwarenessLgbtqTerms,
            SocialAwarenessLgbtqOffensive,
            SocialAwarenessNword,
            SocialAwarenessSexism,
            SpellingAbleAtable,
            SpellingAbleIble,
            SpellingAllyLy,
            SpellingAnceEnce,
            SpellingAthletes,
            SpellingConsistency,
            SpellingElyLy,
            SpellingEmImEnIn,
            SpellingErOr,
            SpellingInUn,
            SpellingMisc,
            SpellingVeOf,
            TermsAnimalAdjectives,
            TermsDenizenLabelsGarner,
            TermsDenizenLabelsNorris,
            TermsEponymousAdjectives,
            TermsVenery,
            TypographyDiacriticalMarks,
            TypographyPunctuationMisplaced,
            TypographyPunctuationExclamation,
            TypographyPunctuationHyperbole,
            TypographyPunctuationSpacing,
            TypographySymbolsEllipsis,
            TypographySymbolsCopyright,
            TypographySymbolsTrademark,
            TypographySymbolsSentenceSpacing,
            TypographySymbolsMultiplication,
            TypographySymbolsCurlyQuotes,
            Uncomparables,
            WeaselWordsVery,
        ]
    }
}

fn supported_checks(checks: &Value) -> Option<Vec<SelectedCheck>> {
    fn check_for_path(path: &[String]) -> Option<SelectedCheck> {
        let path: Vec<&str> = path.iter().flat_map(|segment| segment.split('.')).collect();
        match path.as_slice() {
            ["annotations", "misc"] => Some(SelectedCheck::AnnotationsMisc),
            ["archaism"] => Some(SelectedCheck::Archaism),
            ["cliches", "hell"] => Some(SelectedCheck::ClichesHell),
            ["cliches", "misc", "garner"] => Some(SelectedCheck::ClichesMiscGarner),
            ["cliches", "misc", "gnu_diction"] => Some(SelectedCheck::ClichesMiscGnuDiction),
            ["cliches", "misc", "write_good"] => Some(SelectedCheck::ClichesMiscWriteGood),
            ["malapropisms"] => Some(SelectedCheck::Malapropisms),
            ["skunked_terms"] => Some(SelectedCheck::SkunkedTerms),
            ["social_awareness", "nword"] => Some(SelectedCheck::SocialAwarenessNword),
            ["social_awareness", "lgbtq", "offensive"] => {
                Some(SelectedCheck::SocialAwarenessLgbtqOffensive)
            }
            ["social_awareness", "lgbtq", "terms"] => {
                Some(SelectedCheck::SocialAwarenessLgbtqTerms)
            }
            ["social_awareness", "sexism"] => Some(SelectedCheck::SocialAwarenessSexism),
            ["misc", "apologizing"] => Some(SelectedCheck::MiscApologizing),
            ["misc", "phrasal_adjectives", "ly"] => Some(SelectedCheck::MiscPhrasalAdjectivesLy),
            ["misc", "phrasal_adjectives", "examples"] => {
                Some(SelectedCheck::MiscPhrasalAdjectivesExamples)
            }
            ["misc", "preferred_forms"] => Some(SelectedCheck::MiscPreferredForms),
            ["misc", "but"] => Some(SelectedCheck::MiscBut),
            ["misc", "whence"] => Some(SelectedCheck::MiscWhence),
            ["misc", "waxed"] => Some(SelectedCheck::MiscWaxed),
            ["misc", "illogic"] => Some(SelectedCheck::MiscIllogic),
            ["lexical_illusions"] => Some(SelectedCheck::LexicalIllusions),
            ["misc", "tense_present"] => Some(SelectedCheck::MiscTensePresent),
            ["mixed_metaphors", "bottleneck"] => Some(SelectedCheck::MixedMetaphorsBottleneck),
            ["mixed_metaphors", "misc"] => Some(SelectedCheck::MixedMetaphorsMisc),
            ["redundancy", "ras_syndrome"] => Some(SelectedCheck::RedundancyRasSyndrome),
            ["restricted", "elementary"] => Some(SelectedCheck::RestrictedElementary),
            ["restricted", "top1000"] => Some(SelectedCheck::RestrictedTop1000),
            ["misc", "suddenly"] => Some(SelectedCheck::MiscSuddenly),
            ["misc", "debased"] => Some(SelectedCheck::MiscDebased),
            ["misc", "currency"] => Some(SelectedCheck::MiscCurrency),
            ["misc", "inferior_superior"] => Some(SelectedCheck::MiscInferiorSuperior),
            ["dates_times", "am_pm", "lowercase_periods"] => {
                Some(SelectedCheck::DatesTimesAmPmLowercasePeriods)
            }
            ["dates_times", "am_pm", "spacing"] => Some(SelectedCheck::DatesTimesAmPmSpacing),
            ["dates_times", "am_pm", "midnight_noon"] => {
                Some(SelectedCheck::DatesTimesAmPmMidnightNoon)
            }
            ["dates_times", "dates", "decade_apostrophes"] => {
                Some(SelectedCheck::DatesTimesDatesDecadeApostrophes)
            }
            ["dates_times", "dates", "dash_and_from"] => {
                Some(SelectedCheck::DatesTimesDatesDashAndFrom)
            }
            ["dates_times", "dates", "month_year_comma"] => {
                Some(SelectedCheck::DatesTimesDatesMonthYearComma)
            }
            ["dates_times", "dates", "month_of_year"] => {
                Some(SelectedCheck::DatesTimesDatesMonthOfYear)
            }
            ["misc", "scare_quotes"] => Some(SelectedCheck::MiscScareQuotes),
            ["typography", "punctuation", "misplaced"] => {
                Some(SelectedCheck::TypographyPunctuationMisplaced)
            }
            ["typography", "punctuation", "exclamation"] => {
                Some(SelectedCheck::TypographyPunctuationExclamation)
            }
            ["typography", "punctuation", "hyperbole"] => {
                Some(SelectedCheck::TypographyPunctuationHyperbole)
            }
            ["typography", "punctuation", "spacing"] => {
                Some(SelectedCheck::TypographyPunctuationSpacing)
            }
            ["typography", "diacritical_marks"] => Some(SelectedCheck::TypographyDiacriticalMarks),
            ["misc", "false_plurals", "kudos"] => Some(SelectedCheck::MiscFalsePluralsKudos),
            ["misc", "false_plurals", "misc"] => Some(SelectedCheck::MiscFalsePluralsMisc),
            ["psychology", "mental_telepathy"] => Some(SelectedCheck::PsychologyMentalTelepathy),
            ["psychology", "p_equals_zero"] => Some(SelectedCheck::PsychologyPEqualsZero),
            ["psychology", "lie_detector"] => Some(SelectedCheck::PsychologyLieDetector),
            ["redundancy", "misc", "wallace"] => Some(SelectedCheck::RedundancyMiscWallace),
            ["redundancy", "misc", "garner"] => Some(SelectedCheck::RedundancyMiscGarner),
            ["redundancy", "misc", "nordquist"] => Some(SelectedCheck::RedundancyMiscNordquist),
            ["redundancy", "misc", "after_the_deadline"] => {
                Some(SelectedCheck::RedundancyMiscAfterTheDeadline)
            }
            ["misc", "illogic", "coin"] => Some(SelectedCheck::MiscIllogicCoin),
            ["misc", "illogic", "collusion"] => Some(SelectedCheck::MiscIllogicCollusion),
            ["misc", "back_formations"] => Some(SelectedCheck::MiscBackFormations),
            ["misc", "greylist"] => Some(SelectedCheck::MiscGreylist),
            ["industrial_language", "bureaucratese"] => {
                Some(SelectedCheck::IndustrialLanguageBureaucratese)
            }
            ["industrial_language", "chatspeak"] => {
                Some(SelectedCheck::IndustrialLanguageChatspeak)
            }
            ["industrial_language", "corporate_speak"] => {
                Some(SelectedCheck::IndustrialLanguageCorporateSpeak)
            }
            ["industrial_language", "commercialese"] => {
                Some(SelectedCheck::IndustrialLanguageCommercialese)
            }
            ["hedging"] => Some(SelectedCheck::Hedging),
            ["industrial_language", "jargon"] => Some(SelectedCheck::IndustrialLanguageJargon),
            ["industrial_language", "airlinese"] => {
                Some(SelectedCheck::IndustrialLanguageAirlinese)
            }
            ["misc", "many_a"] => Some(SelectedCheck::MiscManyA),
            ["misc", "metadiscourse"] => Some(SelectedCheck::MiscMetadiscourse),
            ["misc", "narcissism"] => Some(SelectedCheck::MiscNarcissism),
            ["misc", "not_guilty"] => Some(SelectedCheck::MiscNotGuilty),
            ["misc", "pretension"] => Some(SelectedCheck::MiscPretension),
            ["misc", "latin"] => Some(SelectedCheck::MiscLatin),
            ["misc", "capitalization", "seasons"] => Some(SelectedCheck::MiscCapitalizationSeasons),
            ["misc", "capitalization", "terms"] => Some(SelectedCheck::MiscCapitalizationTerms),
            ["misc", "capitalization", "days"] => Some(SelectedCheck::MiscCapitalizationDays),
            ["misc", "capitalization", "months"] => Some(SelectedCheck::MiscCapitalizationMonths),
            ["misc", "institution_name"] => Some(SelectedCheck::MiscInstitutionName),
            ["misc", "professions"] => Some(SelectedCheck::MiscProfessions),
            ["spelling", "ally_ly"] => Some(SelectedCheck::SpellingAllyLy),
            ["spelling", "able_atable"] => Some(SelectedCheck::SpellingAbleAtable),
            ["spelling", "able_ible"] => Some(SelectedCheck::SpellingAbleIble),
            ["spelling", "ance_ence"] => Some(SelectedCheck::SpellingAnceEnce),
            ["spelling", "ely_ly"] => Some(SelectedCheck::SpellingElyLy),
            ["spelling", "er_or"] => Some(SelectedCheck::SpellingErOr),
            ["spelling", "em_im_en_in"] => Some(SelectedCheck::SpellingEmImEnIn),
            ["spelling", "athletes"] => Some(SelectedCheck::SpellingAthletes),
            ["spelling", "consistency"] => Some(SelectedCheck::SpellingConsistency),
            ["spelling", "in_un"] => Some(SelectedCheck::SpellingInUn),
            ["spelling", "misc"] => Some(SelectedCheck::SpellingMisc),
            ["spelling", "ve_of"] => Some(SelectedCheck::SpellingVeOf),
            ["mondegreens"] => Some(SelectedCheck::Mondegreens),
            ["needless_variants"] => Some(SelectedCheck::NeedlessVariants),
            ["oxymorons"] => Some(SelectedCheck::Oxymorons),
            ["misc", "composition"] => Some(SelectedCheck::MiscComposition),
            ["nonwords"] => Some(SelectedCheck::Nonwords),
            ["terms", "eponymous_adjectives"] => Some(SelectedCheck::TermsEponymousAdjectives),
            ["terms", "venery"] => Some(SelectedCheck::TermsVenery),
            ["terms", "animal_adjectives"] => Some(SelectedCheck::TermsAnimalAdjectives),
            ["terms", "denizen_labels", "garner"] => Some(SelectedCheck::TermsDenizenLabelsGarner),
            ["terms", "denizen_labels", "norris"] => Some(SelectedCheck::TermsDenizenLabelsNorris),
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
            ["typography", "symbols", "curly_quotes"] => {
                Some(SelectedCheck::TypographySymbolsCurlyQuotes)
            }
            ["uncomparables"] => Some(SelectedCheck::Uncomparables),
            ["weasel_words", "very"] => Some(SelectedCheck::WeaselWordsVery),
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
                Value::Object(_) if path.as_slice() == ["misc", "illogic"] => {}
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

fn flatten_checks(
    value: &Value,
    prefix: &str,
    per_file: bool,
    checks: &mut Vec<(String, bool, bool)>,
) {
    match value {
        Value::Object(entries) => {
            for (key, value) in entries {
                let path = if prefix.is_empty() {
                    key.clone()
                } else {
                    format!("{prefix}.{key}")
                };
                flatten_checks(value, &path, per_file, checks);
            }
        }
        Value::Bool(enabled) => checks.push((prefix.to_owned(), *enabled, per_file)),
        _ => {}
    }
}

fn selected_check_path(check: SelectedCheck) -> &'static str {
    match check {
        SelectedCheck::AnnotationsMisc => "annotations.misc",
        SelectedCheck::Archaism => "archaism",
        SelectedCheck::ClichesHell => "cliches.hell",
        SelectedCheck::ClichesMiscGarner => "cliches.misc.garner",
        SelectedCheck::ClichesMiscGnuDiction => "cliches.misc.gnu_diction",
        SelectedCheck::ClichesMiscWriteGood => "cliches.misc.write_good",
        SelectedCheck::Malapropisms => "malapropisms",
        SelectedCheck::SkunkedTerms => "skunked_terms",
        SelectedCheck::SocialAwarenessNword => "social_awareness.nword",
        SelectedCheck::SocialAwarenessLgbtqOffensive => "social_awareness.lgbtq.offensive",
        SelectedCheck::SocialAwarenessLgbtqTerms => "social_awareness.lgbtq.terms",
        SelectedCheck::SocialAwarenessSexism => "social_awareness.sexism",
        SelectedCheck::MiscApologizing => "misc.apologizing",
        SelectedCheck::MiscPhrasalAdjectivesLy => "misc.phrasal_adjectives.ly",
        SelectedCheck::MiscPhrasalAdjectivesExamples => "misc.phrasal_adjectives.examples",
        SelectedCheck::MiscPreferredForms => "misc.preferred_forms",
        SelectedCheck::MiscBut => "misc.but",
        SelectedCheck::MiscWhence => "misc.whence",
        SelectedCheck::MiscWaxed => "misc.waxed",
        SelectedCheck::MiscIllogic => "misc.illogic",
        SelectedCheck::LexicalIllusions => "lexical_illusions",
        SelectedCheck::MiscTensePresent => "misc.tense_present",
        SelectedCheck::MixedMetaphorsBottleneck => "mixed_metaphors.bottleneck",
        SelectedCheck::MixedMetaphorsMisc => "mixed_metaphors.misc",
        SelectedCheck::MiscSuddenly => "misc.suddenly",
        SelectedCheck::MiscDebased => "misc.debased",
        SelectedCheck::MiscCurrency => "misc.currency",
        SelectedCheck::MiscInferiorSuperior => "misc.inferior_superior",
        SelectedCheck::DatesTimesAmPmLowercasePeriods => "dates_times.am_pm.lowercase_periods",
        SelectedCheck::DatesTimesAmPmSpacing => "dates_times.am_pm.spacing",
        SelectedCheck::DatesTimesAmPmMidnightNoon => "dates_times.am_pm.midnight_noon",
        SelectedCheck::DatesTimesDatesDecadeApostrophes => "dates_times.dates.decade_apostrophes",
        SelectedCheck::DatesTimesDatesDashAndFrom => "dates_times.dates.dash_and_from",
        SelectedCheck::DatesTimesDatesMonthYearComma => "dates_times.dates.month_year_comma",
        SelectedCheck::DatesTimesDatesMonthOfYear => "dates_times.dates.month_of_year",
        SelectedCheck::MiscScareQuotes => "misc.scare_quotes",
        SelectedCheck::TypographyPunctuationMisplaced => "typography.punctuation.misplaced",
        SelectedCheck::TypographyPunctuationExclamation => "typography.punctuation.exclamation",
        SelectedCheck::TypographyPunctuationHyperbole => "typography.punctuation.hyperbole",
        SelectedCheck::TypographyPunctuationSpacing => "typography.punctuation.spacing",
        SelectedCheck::TypographyDiacriticalMarks => "typography.diacritical_marks",
        SelectedCheck::MiscFalsePluralsKudos => "misc.false_plurals.kudos",
        SelectedCheck::MiscFalsePluralsMisc => "misc.false_plurals.misc",
        SelectedCheck::PsychologyMentalTelepathy => "psychology.mental_telepathy",
        SelectedCheck::PsychologyPEqualsZero => "psychology.p_equals_zero",
        SelectedCheck::PsychologyLieDetector => "psychology.lie_detector",
        SelectedCheck::RedundancyMiscWallace => "redundancy.misc.wallace",
        SelectedCheck::RedundancyMiscGarner => "redundancy.misc.garner",
        SelectedCheck::RedundancyMiscNordquist => "redundancy.misc.nordquist",
        SelectedCheck::RedundancyMiscAfterTheDeadline => "redundancy.misc.after_the_deadline",
        SelectedCheck::RedundancyRasSyndrome => "redundancy.ras_syndrome",
        SelectedCheck::RestrictedElementary => "restricted.elementary",
        SelectedCheck::RestrictedTop1000 => "restricted.top1000",
        SelectedCheck::MiscIllogicCoin => "misc.illogic.coin",
        SelectedCheck::MiscIllogicCollusion => "misc.illogic.collusion",
        SelectedCheck::MiscBackFormations => "misc.back_formations",
        SelectedCheck::MiscGreylist => "misc.greylist",
        SelectedCheck::IndustrialLanguageBureaucratese => "industrial_language.bureaucratese",
        SelectedCheck::IndustrialLanguageChatspeak => "industrial_language.chatspeak",
        SelectedCheck::IndustrialLanguageCommercialese => "industrial_language.commercialese",
        SelectedCheck::IndustrialLanguageCorporateSpeak => "industrial_language.corporate_speak",
        SelectedCheck::Hedging => "hedging",
        SelectedCheck::IndustrialLanguageJargon => "industrial_language.jargon",
        SelectedCheck::IndustrialLanguageAirlinese => "industrial_language.airlinese",
        SelectedCheck::MiscManyA => "misc.many_a",
        SelectedCheck::MiscMetadiscourse => "misc.metadiscourse",
        SelectedCheck::MiscNarcissism => "misc.narcissism",
        SelectedCheck::MiscNotGuilty => "misc.not_guilty",
        SelectedCheck::MiscPretension => "misc.pretension",
        SelectedCheck::MiscLatin => "misc.latin",
        SelectedCheck::MiscCapitalizationSeasons => "misc.capitalization.seasons",
        SelectedCheck::MiscCapitalizationTerms => "misc.capitalization.terms",
        SelectedCheck::MiscCapitalizationDays => "misc.capitalization.days",
        SelectedCheck::MiscCapitalizationMonths => "misc.capitalization.months",
        SelectedCheck::MiscInstitutionName => "misc.institution_name",
        SelectedCheck::MiscProfessions => "misc.professions",
        SelectedCheck::SpellingAbleAtable => "spelling.able_atable",
        SelectedCheck::SpellingAbleIble => "spelling.able_ible",
        SelectedCheck::SpellingAllyLy => "spelling.ally_ly",
        SelectedCheck::SpellingAnceEnce => "spelling.ance_ence",
        SelectedCheck::SpellingElyLy => "spelling.ely_ly",
        SelectedCheck::SpellingErOr => "spelling.er_or",
        SelectedCheck::SpellingEmImEnIn => "spelling.em_im_en_in",
        SelectedCheck::SpellingAthletes => "spelling.athletes",
        SelectedCheck::SpellingConsistency => "spelling.consistency",
        SelectedCheck::SpellingInUn => "spelling.in_un",
        SelectedCheck::SpellingMisc => "spelling.misc",
        SelectedCheck::SpellingVeOf => "spelling.ve_of",
        SelectedCheck::Mondegreens => "mondegreens",
        SelectedCheck::NeedlessVariants => "needless_variants",
        SelectedCheck::Nonwords => "nonwords",
        SelectedCheck::Oxymorons => "oxymorons",
        SelectedCheck::MiscComposition => "misc.composition",
        SelectedCheck::TermsEponymousAdjectives => "terms.eponymous_adjectives",
        SelectedCheck::TermsVenery => "terms.venery",
        SelectedCheck::TermsAnimalAdjectives => "terms.animal_adjectives",
        SelectedCheck::TermsDenizenLabelsGarner => "terms.denizen_labels.garner",
        SelectedCheck::TermsDenizenLabelsNorris => "terms.denizen_labels.norris",
        SelectedCheck::TypographySymbolsEllipsis => "typography.symbols.ellipsis",
        SelectedCheck::TypographySymbolsCopyright => "typography.symbols.copyright",
        SelectedCheck::TypographySymbolsSentenceSpacing => "typography.symbols.sentence_spacing",
        SelectedCheck::TypographySymbolsTrademark => "typography.symbols.trademark",
        SelectedCheck::TypographySymbolsMultiplication => "typography.symbols.multiplication",
        SelectedCheck::TypographySymbolsCurlyQuotes => "typography.symbols.curly_quotes",
        SelectedCheck::Uncomparables => "uncomparables",
        SelectedCheck::WeaselWordsVery => "weasel_words.very",
    }
}

fn default_supported_checks(checks: &Value) -> Vec<SelectedCheck> {
    let mut flattened = Vec::new();
    flatten_checks(checks, "", false, &mut flattened);
    SelectedCheck::all_implemented()
        .into_iter()
        .filter(|check| {
            let path = selected_check_path(*check);
            flattened
                .iter()
                .enumerate()
                .filter(|(_, (candidate, _, _))| {
                    path == candidate || path.starts_with(&format!("{candidate}."))
                })
                .max_by_key(|(index, (candidate, _, _))| (candidate.matches('.').count(), *index))
                .is_some_and(|(_, (_, enabled, _))| *enabled)
        })
        .collect()
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

fn expand_input_paths(paths: &[PathBuf]) -> Vec<PathBuf> {
    const EXTENSIONS: [&str; 6] = ["md", "txt", "rtf", "html", "tex", "markdown"];

    fn visit(path: &std::path::Path, output: &mut Vec<PathBuf>) {
        if !path.is_dir() {
            output.push(path.to_path_buf());
            return;
        }

        // Match os.walk's default behavior: keep the filesystem's enumeration order,
        // recurse into real directories, and ignore traversal errors.
        let Ok(entries) = fs::read_dir(path) else {
            return;
        };
        for entry in entries.flatten() {
            let child = entry.path();
            let Ok(file_type) = entry.file_type() else {
                continue;
            };
            if file_type.is_dir() {
                visit(&child, output);
            } else if file_type.is_symlink()
                && fs::metadata(&child).is_ok_and(|metadata| metadata.is_dir())
            {
                continue;
            } else if child
                .extension()
                .and_then(|extension| extension.to_str())
                .is_some_and(|extension| EXTENSIONS.contains(&extension))
            {
                output.push(child);
            }
        }
    }

    let mut expanded = Vec::new();
    for path in paths {
        visit(path, &mut expanded);
    }
    expanded
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

enum ResolutionPart {
    Prefix(OsString),
    RootDir,
    CurDir,
    ParentDir,
    Normal(OsString),
    EndSymlink(PathBuf),
}

fn resolution_parts(path: &std::path::Path) -> VecDeque<ResolutionPart> {
    path.components()
        .map(|component| match component {
            std::path::Component::Prefix(prefix) => {
                ResolutionPart::Prefix(prefix.as_os_str().to_os_string())
            }
            std::path::Component::RootDir => ResolutionPart::RootDir,
            std::path::Component::CurDir => ResolutionPart::CurDir,
            std::path::Component::ParentDir => ResolutionPart::ParentDir,
            std::path::Component::Normal(name) => ResolutionPart::Normal(name.to_os_string()),
        })
        .collect()
}

fn resolved_path(path: &std::path::Path) -> Result<PathBuf, io::Error> {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()?.join(path)
    };
    let mut pending = resolution_parts(&absolute);
    let mut resolved = PathBuf::new();
    let mut resolving_symlinks = HashSet::new();

    while let Some(part) = pending.pop_front() {
        match part {
            ResolutionPart::Prefix(prefix) => resolved.push(prefix),
            ResolutionPart::RootDir => resolved.push(std::path::MAIN_SEPARATOR.to_string()),
            ResolutionPart::CurDir => {}
            ResolutionPart::ParentDir => {
                resolved.pop();
            }
            ResolutionPart::EndSymlink(path) => {
                resolving_symlinks.remove(&path);
            }
            ResolutionPart::Normal(name) => {
                let candidate = resolved.join(&name);
                if fs::symlink_metadata(&candidate)
                    .is_ok_and(|metadata| metadata.file_type().is_symlink())
                {
                    let parent = resolved.canonicalize().unwrap_or_else(|_| resolved.clone());
                    let link = parent.join(&name);
                    if !resolving_symlinks.insert(link.clone()) {
                        return Err(io::Error::other("symbolic link loop while resolving path"));
                    }
                    let target = fs::read_link(candidate)?;
                    if target.is_absolute() {
                        resolved = PathBuf::new();
                    }
                    let mut expanded = resolution_parts(&target);
                    expanded.push_back(ResolutionPart::EndSymlink(link));
                    expanded.append(&mut pending);
                    pending = expanded;
                } else if candidate.exists() {
                    resolved = candidate.canonicalize()?;
                } else {
                    resolved.push(name);
                }
            }
        }
    }

    Ok(resolved)
}
