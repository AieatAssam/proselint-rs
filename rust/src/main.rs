use clap::{Parser, Subcommand, ValueEnum};
use proselint_rs::{
    airlinese_diagnostics, am_pm_lowercase_periods_diagnostics, am_pm_spacing_diagnostics,
    annotation_diagnostics_in_matcher_order, apologizing_diagnostics, back_formations_diagnostics,
    bureaucratese_diagnostics, but_diagnostics, capitalization_seasons_diagnostics,
    capitalization_terms_diagnostics, cliches_hell_diagnostics, currency_diagnostics,
    debased_diagnostics, false_plurals_kudos_diagnostics,
    false_plurals_misc_diagnostics_in_matcher_order, file_uri, greylist_diagnostics,
    hedging_diagnostics, illogic_coin_diagnostics, illogic_collusion_diagnostics,
    inferior_superior_diagnostics, institution_name_diagnostics, jargon_diagnostics, json_result,
    latin_diagnostics, lie_detector_diagnostics, many_a_diagnostics, mental_telepathy_diagnostics,
    metadiscourse_diagnostics, normalize_file_newlines, pretension_diagnostics,
    professions_diagnostics, suddenly_diagnostics, typography_copyright_diagnostics,
    typography_ellipsis_diagnostics, typography_multiplication_diagnostics,
    typography_sentence_spacing_diagnostics, typography_trademark_diagnostics, wallace_diagnostics,
    whence_diagnostics,
};
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

struct JsonScanner {
    chars: Vec<char>,
    offset: usize,
    failure: Option<JsonSyntaxError>,
}

impl JsonScanner {
    fn new(source: &str) -> Self {
        Self {
            chars: source.chars().collect(),
            offset: 0,
            failure: None,
        }
    }

    fn scan(mut self) -> Option<JsonSyntaxError> {
        self.space();
        if self.offset == self.chars.len() {
            return Some(self.error("Expecting value", self.offset));
        }
        if self.value().is_err() {
            return self.failure;
        }
        self.space();
        (self.offset != self.chars.len()).then(|| self.error("Extra data", self.offset))
    }

    fn value(&mut self) -> Result<(), ()> {
        match self.peek() {
            Some('{') => self.object(),
            Some('[') => self.array(),
            Some('"') => self.string().map(|_| ()),
            Some('t') => self.literal("true"),
            Some('f') => self.literal("false"),
            Some('n') => self.literal("null"),
            Some('N') => self.literal("NaN"),
            Some('I') => self.literal("Infinity"),
            Some('-') if self.chars.get(self.offset + 1) == Some(&'I') => self.literal("-Infinity"),
            Some('-' | '0'..='9') => self.number(),
            _ => self.fail("Expecting value", self.offset),
        }
    }

    fn object(&mut self) -> Result<(), ()> {
        self.offset += 1;
        self.space();
        if self.take('}') {
            return Ok(());
        }
        loop {
            if self.peek() != Some('"') {
                return self.fail(
                    "Expecting property name enclosed in double quotes",
                    self.offset,
                );
            }
            self.string()?;
            self.space();
            if !self.take(':') {
                return self.fail("Expecting ':' delimiter", self.offset);
            }
            self.space();
            self.value()?;
            self.space();
            if self.take('}') {
                return Ok(());
            }
            if !self.take(',') {
                return self.fail("Expecting ',' delimiter", self.offset);
            }
            self.space();
        }
    }

    fn array(&mut self) -> Result<(), ()> {
        self.offset += 1;
        self.space();
        if self.take(']') {
            return Ok(());
        }
        loop {
            self.value()?;
            self.space();
            if self.take(']') {
                return Ok(());
            }
            if !self.take(',') {
                return self.fail("Expecting ',' delimiter", self.offset);
            }
            self.space();
        }
    }

    fn string(&mut self) -> Result<(), ()> {
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
                                    return self.fail("Invalid \\uXXXX escape", slash + 1);
                                }
                                self.offset += 1;
                            }
                        }
                        Some(_) => return self.fail("Invalid \\escape", slash),
                        None => return self.fail("Unterminated string starting at", start),
                    }
                }
                ch if ch <= '\u{1f}' => {
                    return self.fail("Invalid control character at", self.offset);
                }
                _ => self.offset += 1,
            }
        }
        self.fail("Unterminated string starting at", start)
    }

    fn number(&mut self) -> Result<(), ()> {
        let start = self.offset;
        if self.take('-') && self.offset == self.chars.len() {
            return self.fail("Expecting value", start);
        }
        if self.take('0') {
            // Python leaves a leading zero's following digit for the delimiter check.
        } else if self.peek().is_some_and(|c| ('1'..='9').contains(&c)) {
            self.offset += 1;
            while self.peek().is_some_and(|c| c.is_ascii_digit()) {
                self.offset += 1;
            }
        } else {
            return self.fail("Expecting value", start);
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

    fn literal(&mut self, literal: &str) -> Result<(), ()> {
        let start = self.offset;
        for expected in literal.chars() {
            if self.peek() != Some(expected) {
                return self.fail("Expecting value", start);
            }
            self.offset += 1;
        }
        Ok(())
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
    fn fail(&mut self, reason: &'static str, offset: usize) -> Result<(), ()> {
        self.failure = Some(self.error(reason, offset));
        Err(())
    }
}

fn python_json_error_message(source: &str) -> Option<String> {
    let normalized = source.replace("\r\n", "\n").replace('\r', "\n");
    let error = if normalized.starts_with('\u{feff}') {
        JsonSyntaxError {
            reason: "Unexpected UTF-8 BOM (decode using utf-8-sig)",
            offset: 0,
        }
    } else {
        JsonScanner::new(&normalized).scan()?
    };
    let prefix: Vec<char> = normalized.chars().take(error.offset).collect();
    let line = prefix.iter().filter(|&&ch| ch == '\n').count() + 1;
    let column = prefix.iter().rev().take_while(|&&ch| ch != '\n').count() + 1;
    Some(format!(
        "{}: line {line} column {column} (char {})",
        error.reason, error.offset
    ))
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
        config: PathBuf,
        #[arg(long = "output-format", short = 'o', value_enum)]
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
    let config_value: Value = match serde_json::from_str(&config_text) {
        Ok(value) => value,
        Err(error) => {
            if matches!(output_format, OutputFormat::Json) {
                let Some(message) = python_json_error_message(&config_text) else {
                    return Err(format!(
                        "Configuration file '{}' was rejected by serde_json even though the Python JSON scanner found no syntax error: {error}",
                        config.display()
                    ));
                };
                println!(
                    "{}",
                    serde_json::json!({"error":{"code":-31999,"message":message,"data":"JSONDecodeError"}})
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
    let max_errors = config_value
        .get("max_errors")
        .and_then(Value::as_u64)
        .and_then(|limit| usize::try_from(limit).ok())
        .unwrap_or(0);

    if !matches!(output_format, OutputFormat::Json) {
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

    let mut effective_checks = checks.clone();
    if let Some(path) = paths.first()
        && let Some(per_file_checks) = config_value
            .get("per_file_checks")
            .and_then(Value::as_object)
        && let Some((_, override_checks)) = per_file_checks
            .iter()
            .find(|(pattern, _)| path_matches_pattern(path, pattern))
        && let Some(global_selected) = supported_checks(checks)
        && effective_check_enabled(
            checks,
            override_checks,
            selected_check_path(global_selected),
        ) == Some(false)
    {
        effective_checks = serde_json::json!({});
    }
    let selected_check = supported_checks(&effective_checks).ok_or_else(|| {
        "this capability supports disabled checks, annotations.misc, cliches.hell, misc.apologizing, misc.but, misc.whence, misc.suddenly, misc.debased, misc.currency, misc.inferior_superior, misc.false_plurals.kudos, psychology.mental_telepathy, psychology.lie_detector, misc.illogic.coin, misc.illogic.collusion, misc.back_formations, misc.many_a, misc.metadiscourse, misc.pretension, misc.latin, misc.capitalization.seasons, misc.capitalization.terms, misc.institution_name, misc.professions, misc.greylist, redundancy.misc.wallace, industrial_language.bureaucratese, industrial_language.jargon, industrial_language.airlinese, hedging, typography.symbols.ellipsis, typography.symbols.copyright, typography.symbols.sentence_spacing, typography.symbols.trademark, typography.symbols.multiplication, dates_times.am_pm.lowercase_periods, or dates_times.am_pm.spacing only".to_owned()
    })?;

    let diagnostics = match selected_check {
        SelectedCheck::None => Vec::new(),
        SelectedCheck::AnnotationsMisc => truncate_then_sort(
            annotation_diagnostics_in_matcher_order(&content),
            max_errors,
        ),
        SelectedCheck::ClichesHell => cliches_hell_diagnostics(&content),
        SelectedCheck::MiscApologizing => apologizing_diagnostics(&content),
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
        SelectedCheck::MiscFalsePluralsMisc => truncate_then_sort(
            false_plurals_misc_diagnostics_in_matcher_order(&content),
            max_errors,
        ),
        SelectedCheck::PsychologyMentalTelepathy => mental_telepathy_diagnostics(&content),
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
        SelectedCheck::MiscCapitalizationSeasons => capitalization_seasons_diagnostics(&content),
        SelectedCheck::MiscCapitalizationTerms => capitalization_terms_diagnostics(&content),
        SelectedCheck::MiscInstitutionName => institution_name_diagnostics(&content),
        SelectedCheck::MiscProfessions => professions_diagnostics(&content),
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
    let diagnostics = if matches!(
        selected_check,
        SelectedCheck::AnnotationsMisc | SelectedCheck::MiscFalsePluralsMisc
    ) {
        diagnostics
    } else {
        diagnostics.into_iter().take(max_errors).collect()
    };
    let exit_code = if diagnostics.is_empty() {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    };
    let result = json_result(&source, diagnostics);
    println!("{result}");
    Ok(exit_code)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SelectedCheck {
    None,
    AnnotationsMisc,
    ClichesHell,
    MiscApologizing,
    MiscBut,
    MiscWhence,
    MiscSuddenly,
    MiscDebased,
    MiscCurrency,
    MiscInferiorSuperior,
    DatesTimesAmPmLowercasePeriods,
    DatesTimesAmPmSpacing,
    MiscFalsePluralsKudos,
    MiscFalsePluralsMisc,
    PsychologyMentalTelepathy,
    PsychologyLieDetector,
    RedundancyMiscWallace,
    MiscIllogicCoin,
    MiscIllogicCollusion,
    MiscBackFormations,
    MiscGreylist,
    IndustrialLanguageBureaucratese,
    Hedging,
    IndustrialLanguageJargon,
    IndustrialLanguageAirlinese,
    MiscManyA,
    MiscMetadiscourse,
    MiscPretension,
    MiscLatin,
    MiscCapitalizationSeasons,
    MiscCapitalizationTerms,
    MiscInstitutionName,
    MiscProfessions,
    TypographySymbolsEllipsis,
    TypographySymbolsCopyright,
    TypographySymbolsSentenceSpacing,
    TypographySymbolsTrademark,
    TypographySymbolsMultiplication,
}

fn supported_checks(checks: &Value) -> Option<SelectedCheck> {
    fn visit(value: &Value, path: &mut Vec<String>, selected: &mut SelectedCheck) -> bool {
        match value {
            Value::Object(entries) => entries.iter().all(|(key, child)| {
                path.push(key.clone());
                let result = if path == &["annotations", "misc"] {
                    match child {
                        Value::Bool(false) => true,
                        Value::Bool(true) if *selected == SelectedCheck::None => {
                            *selected = SelectedCheck::AnnotationsMisc;
                            true
                        }
                        _ => false,
                    }
                } else if path == &["cliches", "hell"] {
                    match child {
                        Value::Bool(false) => true,
                        Value::Bool(true) if *selected == SelectedCheck::None => {
                            *selected = SelectedCheck::ClichesHell;
                            true
                        }
                        _ => false,
                    }
                } else if path == &["misc", "apologizing"] {
                    match child {
                        Value::Bool(false) => true,
                        Value::Bool(true) if *selected == SelectedCheck::None => {
                            *selected = SelectedCheck::MiscApologizing;
                            true
                        }
                        _ => false,
                    }
                } else if path == &["misc", "but"] {
                    match child {
                        Value::Bool(false) => true,
                        Value::Bool(true) if *selected == SelectedCheck::None => {
                            *selected = SelectedCheck::MiscBut;
                            true
                        }
                        _ => false,
                    }
                } else if path == &["misc", "whence"] {
                    match child {
                        Value::Bool(false) => true,
                        Value::Bool(true) if *selected == SelectedCheck::None => {
                            *selected = SelectedCheck::MiscWhence;
                            true
                        }
                        _ => false,
                    }
                } else if path == &["misc", "suddenly"] {
                    match child {
                        Value::Bool(false) => true,
                        Value::Bool(true) if *selected == SelectedCheck::None => {
                            *selected = SelectedCheck::MiscSuddenly;
                            true
                        }
                        _ => false,
                    }
                } else if path == &["misc", "debased"] {
                    match child {
                        Value::Bool(false) => true,
                        Value::Bool(true) if *selected == SelectedCheck::None => {
                            *selected = SelectedCheck::MiscDebased;
                            true
                        }
                        _ => false,
                    }
                } else if path == &["misc", "currency"] {
                    match child {
                        Value::Bool(false) => true,
                        Value::Bool(true) if *selected == SelectedCheck::None => {
                            *selected = SelectedCheck::MiscCurrency;
                            true
                        }
                        _ => false,
                    }
                } else if path == &["misc", "inferior_superior"] {
                    match child {
                        Value::Bool(false) => true,
                        Value::Bool(true) if *selected == SelectedCheck::None => {
                            *selected = SelectedCheck::MiscInferiorSuperior;
                            true
                        }
                        _ => false,
                    }
                } else if path == &["dates_times", "am_pm", "lowercase_periods"] {
                    match child {
                        Value::Bool(false) => true,
                        Value::Bool(true) if *selected == SelectedCheck::None => {
                            *selected = SelectedCheck::DatesTimesAmPmLowercasePeriods;
                            true
                        }
                        _ => false,
                    }
                } else if path == &["dates_times", "am_pm", "spacing"] {
                    match child {
                        Value::Bool(false) => true,
                        Value::Bool(true) if *selected == SelectedCheck::None => {
                            *selected = SelectedCheck::DatesTimesAmPmSpacing;
                            true
                        }
                        _ => false,
                    }
                } else if path == &["misc", "false_plurals", "kudos"] {
                    match child {
                        Value::Bool(false) => true,
                        Value::Bool(true) if *selected == SelectedCheck::None => {
                            *selected = SelectedCheck::MiscFalsePluralsKudos;
                            true
                        }
                        _ => false,
                    }
                } else if path == &["misc", "false_plurals", "misc"] {
                    match child {
                        Value::Bool(false) => true,
                        Value::Bool(true) if *selected == SelectedCheck::None => {
                            *selected = SelectedCheck::MiscFalsePluralsMisc;
                            true
                        }
                        _ => false,
                    }
                } else if path == &["psychology", "mental_telepathy"] {
                    match child {
                        Value::Bool(false) => true,
                        Value::Bool(true) if *selected == SelectedCheck::None => {
                            *selected = SelectedCheck::PsychologyMentalTelepathy;
                            true
                        }
                        _ => false,
                    }
                } else if path == &["psychology", "lie_detector"] {
                    match child {
                        Value::Bool(false) => true,
                        Value::Bool(true) if *selected == SelectedCheck::None => {
                            *selected = SelectedCheck::PsychologyLieDetector;
                            true
                        }
                        _ => false,
                    }
                } else if path == &["redundancy", "misc", "wallace"] {
                    match child {
                        Value::Bool(false) => true,
                        Value::Bool(true) if *selected == SelectedCheck::None => {
                            *selected = SelectedCheck::RedundancyMiscWallace;
                            true
                        }
                        _ => false,
                    }
                } else if path == &["misc", "illogic", "coin"] {
                    match child {
                        Value::Bool(false) => true,
                        Value::Bool(true) if *selected == SelectedCheck::None => {
                            *selected = SelectedCheck::MiscIllogicCoin;
                            true
                        }
                        _ => false,
                    }
                } else if path == &["misc", "illogic", "collusion"] {
                    match child {
                        Value::Bool(false) => true,
                        Value::Bool(true) if *selected == SelectedCheck::None => {
                            *selected = SelectedCheck::MiscIllogicCollusion;
                            true
                        }
                        _ => false,
                    }
                } else if path == &["misc", "back_formations"] {
                    match child {
                        Value::Bool(false) => true,
                        Value::Bool(true) if *selected == SelectedCheck::None => {
                            *selected = SelectedCheck::MiscBackFormations;
                            true
                        }
                        _ => false,
                    }
                } else if path == &["misc", "greylist"] {
                    match child {
                        Value::Bool(false) => true,
                        Value::Bool(true) if *selected == SelectedCheck::None => {
                            *selected = SelectedCheck::MiscGreylist;
                            true
                        }
                        _ => false,
                    }
                } else if path == &["industrial_language", "bureaucratese"] {
                    match child {
                        Value::Bool(false) => true,
                        Value::Bool(true) if *selected == SelectedCheck::None => {
                            *selected = SelectedCheck::IndustrialLanguageBureaucratese;
                            true
                        }
                        _ => false,
                    }
                } else if path == &["hedging"] {
                    match child {
                        Value::Bool(false) => true,
                        Value::Bool(true) if *selected == SelectedCheck::None => {
                            *selected = SelectedCheck::Hedging;
                            true
                        }
                        _ => false,
                    }
                } else if path == &["industrial_language", "jargon"] {
                    match child {
                        Value::Bool(false) => true,
                        Value::Bool(true) if *selected == SelectedCheck::None => {
                            *selected = SelectedCheck::IndustrialLanguageJargon;
                            true
                        }
                        _ => false,
                    }
                } else if path == &["industrial_language", "airlinese"] {
                    match child {
                        Value::Bool(false) => true,
                        Value::Bool(true) if *selected == SelectedCheck::None => {
                            *selected = SelectedCheck::IndustrialLanguageAirlinese;
                            true
                        }
                        _ => false,
                    }
                } else if path == &["misc", "many_a"] {
                    match child {
                        Value::Bool(false) => true,
                        Value::Bool(true) if *selected == SelectedCheck::None => {
                            *selected = SelectedCheck::MiscManyA;
                            true
                        }
                        _ => false,
                    }
                } else if path == &["misc", "metadiscourse"] {
                    match child {
                        Value::Bool(false) => true,
                        Value::Bool(true) if *selected == SelectedCheck::None => {
                            *selected = SelectedCheck::MiscMetadiscourse;
                            true
                        }
                        _ => false,
                    }
                } else if path == &["misc", "pretension"] {
                    match child {
                        Value::Bool(false) => true,
                        Value::Bool(true) if *selected == SelectedCheck::None => {
                            *selected = SelectedCheck::MiscPretension;
                            true
                        }
                        _ => false,
                    }
                } else if path == &["misc", "latin"] {
                    match child {
                        Value::Bool(false) => true,
                        Value::Bool(true) if *selected == SelectedCheck::None => {
                            *selected = SelectedCheck::MiscLatin;
                            true
                        }
                        _ => false,
                    }
                } else if path == &["misc", "capitalization", "seasons"] {
                    match child {
                        Value::Bool(false) => true,
                        Value::Bool(true) if *selected == SelectedCheck::None => {
                            *selected = SelectedCheck::MiscCapitalizationSeasons;
                            true
                        }
                        _ => false,
                    }
                } else if path == &["misc", "capitalization", "terms"] {
                    match child {
                        Value::Bool(false) => true,
                        Value::Bool(true) if *selected == SelectedCheck::None => {
                            *selected = SelectedCheck::MiscCapitalizationTerms;
                            true
                        }
                        _ => false,
                    }
                } else if path == &["misc", "institution_name"] {
                    match child {
                        Value::Bool(false) => true,
                        Value::Bool(true) if *selected == SelectedCheck::None => {
                            *selected = SelectedCheck::MiscInstitutionName;
                            true
                        }
                        _ => false,
                    }
                } else if path == &["misc", "professions"] {
                    match child {
                        Value::Bool(false) => true,
                        Value::Bool(true) if *selected == SelectedCheck::None => {
                            *selected = SelectedCheck::MiscProfessions;
                            true
                        }
                        _ => false,
                    }
                } else if path == &["typography", "symbols", "ellipsis"] {
                    match child {
                        Value::Bool(false) => true,
                        Value::Bool(true) if *selected == SelectedCheck::None => {
                            *selected = SelectedCheck::TypographySymbolsEllipsis;
                            true
                        }
                        _ => false,
                    }
                } else if path == &["typography", "symbols", "copyright"] {
                    match child {
                        Value::Bool(false) => true,
                        Value::Bool(true) if *selected == SelectedCheck::None => {
                            *selected = SelectedCheck::TypographySymbolsCopyright;
                            true
                        }
                        _ => false,
                    }
                } else if path == &["typography", "symbols", "sentence_spacing"] {
                    match child {
                        Value::Bool(false) => true,
                        Value::Bool(true) if *selected == SelectedCheck::None => {
                            *selected = SelectedCheck::TypographySymbolsSentenceSpacing;
                            true
                        }
                        _ => false,
                    }
                } else if path == &["typography", "symbols", "trademark"] {
                    match child {
                        Value::Bool(false) => true,
                        Value::Bool(true) if *selected == SelectedCheck::None => {
                            *selected = SelectedCheck::TypographySymbolsTrademark;
                            true
                        }
                        _ => false,
                    }
                } else if path == &["typography", "symbols", "multiplication"] {
                    match child {
                        Value::Bool(false) => true,
                        Value::Bool(true) if *selected == SelectedCheck::None => {
                            *selected = SelectedCheck::TypographySymbolsMultiplication;
                            true
                        }
                        _ => false,
                    }
                } else {
                    visit(child, path, selected)
                };
                path.pop();
                result
            }),
            Value::Bool(false) => true,
            _ => false,
        }
    }

    if !checks.is_object() {
        return None;
    }
    let mut selected = SelectedCheck::None;
    visit(checks, &mut Vec::new(), &mut selected).then_some(selected)
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

fn truncate_then_sort(mut diagnostics: Vec<Value>, max_errors: usize) -> Vec<Value> {
    diagnostics.truncate(max_errors);
    diagnostics.sort_by_key(|diagnostic| {
        (
            diagnostic["pos"][0].as_u64().unwrap_or_default(),
            diagnostic["pos"][1].as_u64().unwrap_or_default(),
        )
    });
    diagnostics
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
        SelectedCheck::None => "",
        SelectedCheck::AnnotationsMisc => "annotations.misc",
        SelectedCheck::ClichesHell => "cliches.hell",
        SelectedCheck::MiscApologizing => "misc.apologizing",
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
        SelectedCheck::MiscInstitutionName => "misc.institution_name",
        SelectedCheck::MiscProfessions => "misc.professions",
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
