use clap::{Parser, Subcommand, ValueEnum};
use proselint_rs::{
    airlinese_diagnostics, am_pm_lowercase_periods_diagnostics, am_pm_spacing_diagnostics,
    annotation_diagnostics, apologizing_diagnostics, back_formations_diagnostics,
    bureaucratese_diagnostics, but_diagnostics, cliches_hell_diagnostics, currency_diagnostics,
    debased_diagnostics, false_plurals_kudos_diagnostics, false_plurals_misc_diagnostics, file_uri,
    greylist_diagnostics, hedging_diagnostics, illogic_coin_diagnostics,
    illogic_collusion_diagnostics, inferior_superior_diagnostics, institution_name_diagnostics,
    jargon_diagnostics, json_result, many_a_diagnostics, mental_telepathy_diagnostics,
    normalize_file_newlines, professions_diagnostics, suddenly_diagnostics,
    typography_copyright_diagnostics, typography_ellipsis_diagnostics,
    typography_multiplication_diagnostics, typography_sentence_spacing_diagnostics,
    typography_trademark_diagnostics, whence_diagnostics,
};
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
    let selected_check = supported_checks(checks).ok_or_else(|| {
        "this capability supports disabled checks, annotations.misc, cliches.hell, misc.apologizing, misc.but, misc.whence, misc.suddenly, misc.debased, misc.currency, misc.inferior_superior, misc.false_plurals.kudos, psychology.mental_telepathy, misc.illogic.coin, misc.illogic.collusion, misc.back_formations, misc.many_a, misc.institution_name, misc.professions, misc.greylist, industrial_language.bureaucratese, industrial_language.jargon, industrial_language.airlinese, hedging, typography.symbols.ellipsis, typography.symbols.copyright, typography.symbols.sentence_spacing, typography.symbols.trademark, typography.symbols.multiplication, dates_times.am_pm.lowercase_periods, or dates_times.am_pm.spacing only".to_owned()
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

    let diagnostics = match selected_check {
        SelectedCheck::None => Vec::new(),
        SelectedCheck::AnnotationsMisc => annotation_diagnostics(&content),
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
        SelectedCheck::MiscFalsePluralsMisc => false_plurals_misc_diagnostics(&content),
        SelectedCheck::PsychologyMentalTelepathy => mental_telepathy_diagnostics(&content),
        SelectedCheck::MiscIllogicCoin => illogic_coin_diagnostics(&content),
        SelectedCheck::MiscIllogicCollusion => illogic_collusion_diagnostics(&content),
        SelectedCheck::MiscBackFormations => back_formations_diagnostics(&content),
        SelectedCheck::MiscGreylist => greylist_diagnostics(&content),
        SelectedCheck::IndustrialLanguageBureaucratese => bureaucratese_diagnostics(&content),
        SelectedCheck::Hedging => hedging_diagnostics(&content),
        SelectedCheck::IndustrialLanguageJargon => jargon_diagnostics(&content),
        SelectedCheck::IndustrialLanguageAirlinese => airlinese_diagnostics(&content),
        SelectedCheck::MiscManyA => many_a_diagnostics(&content),
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
    MiscIllogicCoin,
    MiscIllogicCollusion,
    MiscBackFormations,
    MiscGreylist,
    IndustrialLanguageBureaucratese,
    Hedging,
    IndustrialLanguageJargon,
    IndustrialLanguageAirlinese,
    MiscManyA,
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
