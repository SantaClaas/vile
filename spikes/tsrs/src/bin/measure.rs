//! Measures loading a project and checking mutants through tsrs, for the report.
//!
//! `measure <tsconfig.json> <path part>`: mutates the project's files whose path contains the
//! part (test files left out) with relational swaps and body replacement, and times each check.

use std::collections::BTreeMap;
use std::process::ExitCode;
use std::time::{Duration, Instant};

use vile_core::patch::Patch;
use vile_core::span::Span;
use vile_spike_tsrs::project::Project;

const RELATIONAL: [(&str, [&str; 2]); 6] = [
    ("<", ["<=", "!=="]),
    ("<=", ["<", "==="]),
    (">", [">=", "!=="]),
    (">=", [">", "==="]),
    ("===", ["<=", ">="]),
    ("!==", ["<", ">"]),
];

/// The values body replacement tries for a return type, as in docs/PLAN.md.
fn find_values(return_type: &str) -> Vec<&'static str> {
    match return_type {
        "void" | "Promise<void>" => vec![""],
        "boolean" => vec!["return true;", "return false;"],
        "number" => vec!["return 0;", "return 1;", "return -1;"],
        "string" => vec!["return \"\";", "return \"xyzzy\";"],
        _ if return_type.ends_with("[]") => vec!["return [];"],
        _ if return_type.contains("| undefined") => vec!["return undefined;"],
        _ => Vec::new(),
    }
}

fn span(start: usize, end: usize) -> Option<Span> {
    Span::new(u32::try_from(start).ok()?, u32::try_from(end).ok()?).ok()
}

/// Relational swaps on operators written with a space on each side, outside comments.
fn find_relational(source: &str) -> Vec<Patch> {
    let mut patches = Vec::new();
    let mut line_start = 0;
    for line in source.split_inclusive('\n') {
        let code = line.trim_start();
        if !(code.starts_with("//") || code.starts_with('*')) {
            for (operator, replacements) in RELATIONAL {
                let needle = format!(" {operator} ");
                for (index, _) in line.match_indices(&needle) {
                    let start = line_start + index + 1;
                    let Some(span) = span(start, start + operator.len()) else {
                        continue;
                    };
                    patches.extend(replacements.iter().map(|replacement| Patch {
                        span,
                        replacement: (*replacement).to_owned(),
                    }));
                }
            }
        }
        line_start += line.len();
    }
    patches
}

fn find_bodies(project: &Project, file: &str) -> Vec<Patch> {
    project
        .functions(file)
        .into_iter()
        .flat_map(|function| {
            find_values(&function.return_type)
                .into_iter()
                .map(move |value| Patch {
                    span: function.body,
                    replacement: if value.is_empty() {
                        "{}".to_owned()
                    } else {
                        format!("{{ {value} }}")
                    },
                })
        })
        .collect()
}

fn read_resident_kilobytes() -> u64 {
    std::fs::read_to_string("/proc/self/status")
        .ok()
        .and_then(|status| {
            let line = status.lines().find(|line| line.starts_with("VmRSS:"))?;
            line.split_whitespace().nth(1)?.parse().ok()
        })
        .unwrap_or(0)
}

fn milliseconds(duration: Duration) -> f64 {
    duration.as_secs_f64() * 1000.0
}

#[expect(
    clippy::print_stdout,
    reason = "the measurements are the program's output"
)]
fn print_times(label: &str, times: &mut [Duration]) {
    times.sort_unstable();
    let Some(&maximum) = times.last() else {
        return;
    };
    let total: Duration = times.iter().sum();
    let count = u32::try_from(times.len()).unwrap_or(u32::MAX);
    println!(
        "{label}: {} checks, median {:.2} ms, p90 {:.2} ms, max {:.2} ms, mean {:.2} ms, total {:.0} ms",
        times.len(),
        milliseconds(times[times.len() / 2]),
        milliseconds(times[times.len() * 9 / 10]),
        milliseconds(maximum),
        milliseconds(total / count),
        milliseconds(total),
    );
}

#[expect(
    clippy::print_stdout,
    reason = "the measurements are the program's output"
)]
#[expect(
    clippy::print_stderr,
    reason = "usage and errors for the person running it"
)]
fn main() -> ExitCode {
    let arguments: Vec<String> = std::env::args().collect();
    let [_, config, part] = arguments.as_slice() else {
        eprintln!("usage: measure <tsconfig.json> <path part>");
        return ExitCode::FAILURE;
    };
    let resident_before = read_resident_kilobytes();
    let started = Instant::now();
    let project = match Project::load(config) {
        Ok(project) => project,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::FAILURE;
        }
    };
    let load = started.elapsed();
    let files: Vec<String> = project
        .file_names()
        .into_iter()
        .filter(|file| file.contains(part.as_str()) && !file.contains(".test."))
        .collect();
    println!(
        "load: {:.0} ms, {} files mutated",
        milliseconds(load),
        files.len()
    );

    let started = Instant::now();
    let mut mutants = Vec::new();
    for file in &files {
        let Some(source) = project.text(file) else {
            continue;
        };
        mutants.extend(
            find_relational(source)
                .into_iter()
                .map(|patch| (file, patch, "relational")),
        );
        mutants.extend(
            find_bodies(&project, file)
                .into_iter()
                .map(|patch| (file, patch, "body")),
        );
    }
    println!(
        "return types and mutants: {:.0} ms for {} mutants",
        milliseconds(started.elapsed()),
        mutants.len()
    );

    let resident_loaded = read_resident_kilobytes();
    let mut first = None;
    let mut times = Vec::new();
    let mut compile_errors = [0_usize; 2];
    let mut counts = [0_usize; 2];
    let mut rebuilt = 0;
    let mut codes: [BTreeMap<i32, usize>; 2] = [BTreeMap::new(), BTreeMap::new()];
    for (file, patch, operator) in &mutants {
        let started = Instant::now();
        let checked = match project.check(file, patch) {
            Ok(checked) => checked,
            Err(error) => {
                eprintln!("{file}: {error}");
                continue;
            }
        };
        let elapsed = started.elapsed();
        if first.is_none() {
            first = Some(elapsed);
        } else {
            times.push(elapsed);
        }
        let index = usize::from(*operator == "body");
        counts[index] += 1;
        compile_errors[index] += usize::from(!checked.diagnostics.is_empty());
        rebuilt += usize::from(!checked.is_reused);
        if let Some(diagnostic) = checked.diagnostics.first() {
            *codes[index].entry(diagnostic.code).or_default() += 1;
        }
    }
    if let Some(first) = first {
        println!("first check: {:.2} ms", milliseconds(first));
    }
    print_times("later checks", &mut times);
    println!(
        "compile errors: relational {}/{}, body replacement {}/{}; programs rebuilt: {rebuilt}",
        compile_errors[0], counts[0], compile_errors[1], counts[1]
    );
    println!(
        "first diagnostic codes: relational {:?}, body replacement {:?}",
        codes[0], codes[1]
    );
    println!(
        "resident memory: {} MB before loading, {} MB loaded, {} MB after the checks",
        resident_before / 1024,
        resident_loaded / 1024,
        read_resident_kilobytes() / 1024
    );
    ExitCode::SUCCESS
}
