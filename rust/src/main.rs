use clap::{Parser, Subcommand, ValueEnum};
use proselint_rs::{clean_json_result, file_uri};
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

fn run() -> Result<(), String> {
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
    if !all_checks_disabled(checks) {
        return Err("this capability handles configurations with all checks disabled".to_owned());
    }

    if !matches!(output_format, OutputFormat::Json) {
        return Err("this capability currently supports JSON output only".to_owned());
    }
    if paths.len() > 1 {
        return Err("this capability currently supports one source at a time".to_owned());
    }

    let source = if let Some(path) = paths.first() {
        // Reading the file establishes the same input-access behavior for the
        // empty-file case. Non-empty linting is implemented by later slices.
        fs::read(path).map_err(|error| format!("Could not read '{}': {error}", path.display()))?;
        file_uri(path)
            .map_err(|error| format!("Could not resolve '{}': {error}", path.display()))?
    } else {
        let mut input = Vec::new();
        io::stdin()
            .read_to_end(&mut input)
            .map_err(|error| format!("Could not read stdin: {error}"))?;
        "<stdin>".to_owned()
    };

    println!("{}", clean_json_result(&source));
    Ok(())
}

fn all_checks_disabled(checks: &Value) -> bool {
    match checks {
        Value::Object(entries) => entries.values().all(all_checks_disabled),
        Value::Bool(false) => true,
        _ => false,
    }
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("{message}");
            ExitCode::from(4)
        }
    }
}
