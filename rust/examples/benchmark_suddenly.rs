use proselint_rs::suddenly_diagnostics;
use serde_json::json;
use std::env;
use std::fs;
use std::process;
use std::time::Instant;

fn main() {
    let mut args = env::args().skip(1);
    let path = args.next().unwrap_or_else(|| {
        eprintln!("usage: benchmark_suddenly <document-path> <repetitions>");
        process::exit(2);
    });
    let repetitions: usize = args
        .next()
        .and_then(|value| value.parse().ok())
        .filter(|value| *value > 0)
        .unwrap_or_else(|| {
            eprintln!("repetitions must be a positive integer");
            process::exit(2);
        });
    if args.next().is_some() {
        eprintln!("usage: benchmark_suddenly <document-path> <repetitions>");
        process::exit(2);
    }

    let document = fs::read_to_string(path).unwrap_or_else(|error| {
        eprintln!("could not read UTF-8 document: {error}");
        process::exit(2);
    });
    let mut first_diagnostics = None;
    let mut total_diagnostics = 0usize;
    let started = Instant::now();
    for index in 0..repetitions {
        let diagnostics = suddenly_diagnostics(&document);
        total_diagnostics += diagnostics.len();
        if index == 0 {
            first_diagnostics = Some(diagnostics);
        }
    }
    let elapsed_ns = started.elapsed().as_nanos();
    let diagnostics = first_diagnostics.expect("positive repetitions produce one iteration");
    println!(
        "{}",
        json!({
            "elapsed_ns": elapsed_ns,
            "repetitions": repetitions,
            "diagnostics_per_iteration": diagnostics.len(),
            "total_diagnostics": total_diagnostics,
            "diagnostics": diagnostics,
        })
    );
}
