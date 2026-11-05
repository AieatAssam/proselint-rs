use clap::{Parser, Subcommand, ValueEnum};
use proselint_rs::{annotation_diagnostics, file_uri, json_result, normalize_file_newlines};
use serde_json::Value;
use std::fs;
use std::io::{self, Read};
use std::path::PathBuf;
use std::process::ExitCode;

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

    let config_text = fs::read_to_string(&config).map_err(|error| {
        format!(
            "Configuration file '{}' could not be read: {error}",
            config.display()
        )
    })?;
    let config_value: Value = serde_json::from_str(&config_text).map_err(|error| {
        format!(
            "Configuration file '{}' is not valid JSON: {error}",
            config.display()
        )
    })?;
    if !config_value.is_object() {
        return Err(format!(
            "Configuration file '{}' must contain a JSON object",
            config.display()
        ));
    }
    let checks = config_value
        .get("checks")
        .ok_or_else(|| "this capability requires an explicit checks configuration".to_owned())?;
    let annotations_enabled = supported_checks(checks).ok_or_else(|| {
        "this capability supports disabled checks or annotations.misc only".to_owned()
    })?;

    if !matches!(output_format, OutputFormat::Json) {
        return Err("this capability currently supports JSON output only".to_owned());
    }
    if paths.len() > 1 {
        return Err("this capability currently supports one source at a time".to_owned());
    }

    let (source, content) = if let Some(path) = paths.first() {
        let content = fs::read_to_string(path)
            .map_err(|error| format!("Could not read '{}': {error}", path.display()))?;
        let content = normalize_file_newlines(&content);
        let source = file_uri(path)
            .map_err(|error| format!("Could not resolve '{}': {error}", path.display()))?;
        (source, content)
    } else {
        let mut input = String::new();
        io::stdin()
            .read_to_string(&mut input)
            .map_err(|error| format!("Could not read stdin: {error}"))?;
        ("<stdin>".to_owned(), input)
    };

    let diagnostics = if annotations_enabled {
        annotation_diagnostics(&content)
    } else {
        Vec::new()
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

fn supported_checks(checks: &Value) -> Option<bool> {
    fn visit(value: &Value, path: &mut Vec<String>, enabled: &mut bool) -> bool {
        match value {
            Value::Object(entries) => entries.iter().all(|(key, child)| {
                path.push(key.clone());
                let result = if path == &["annotations", "misc"] {
                    match child {
                        Value::Bool(false) => true,
                        Value::Bool(true) if !*enabled => {
                            *enabled = true;
                            true
                        }
                        _ => false,
                    }
                } else {
                    visit(child, path, enabled)
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
    let mut enabled = false;
    visit(checks, &mut Vec::new(), &mut enabled).then_some(enabled)
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
