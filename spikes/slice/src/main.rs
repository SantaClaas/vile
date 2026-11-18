//! Writes the mutants of a project's source files to `mutants.json`, for the runner in `runner/`.
//!
//! `vile_spike_slice <project> [<folder or file>...] [--exclude <file>]... [--out <folder>]`
//!
//! The paths are relative to the project and default to `src`. Test files (`*.test.ts`,
//! `*.spec.ts`) and declaration files are never mutated. The output folder defaults to
//! `<project>/vile.out`.
#![expect(
    clippy::multiple_crate_versions,
    reason = "Oxc's dependencies pull in syn 2 (phf_macros) and syn 3 (displaydoc); only they can unify it"
)]

use std::{
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
    process::ExitCode,
};

use vile_core::output::MutantsFile;
use vile_spike_slice::generate;

/// What went wrong, for the person running the spike.
#[derive(Debug, thiserror::Error)]
enum RunError {
    #[error(
        "usage: vile_spike_slice <project> [<folder or file>...] [--exclude <file>]... [--out <folder>]"
    )]
    Usage,
    #[error("can't read {path}: {source}")]
    Read { path: PathBuf, source: io::Error },
    #[error("can't write {path}: {source}")]
    Write { path: PathBuf, source: io::Error },
    #[error("{file}: {source}")]
    Generate {
        file: String,
        source: generate::GenerateError,
    },
}

#[derive(Debug)]
struct Arguments {
    project: PathBuf,
    paths: Vec<PathBuf>,
    excluded: Vec<PathBuf>,
    out: PathBuf,
}

fn parse_arguments(mut arguments: impl Iterator<Item = String>) -> Result<Arguments, RunError> {
    let project = PathBuf::from(arguments.next().ok_or(RunError::Usage)?);
    let mut paths = Vec::new();
    let mut excluded = Vec::new();
    let mut out = None;
    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "--out" => out = Some(PathBuf::from(arguments.next().ok_or(RunError::Usage)?)),
            "--exclude" => excluded.push(project.join(arguments.next().ok_or(RunError::Usage)?)),
            _ => paths.push(project.join(argument)),
        }
    }
    if paths.is_empty() {
        paths.push(project.join("src"));
    }
    let out = out.unwrap_or_else(|| project.join("vile.out"));
    Ok(Arguments {
        project,
        paths,
        excluded,
        out,
    })
}

fn is_mutated(path: &Path) -> bool {
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default();
    let is_source = [".ts", ".tsx", ".mts", ".cts"]
        .iter()
        .any(|extension| name.ends_with(extension));
    let is_test = [".test.", ".spec.", ".d."]
        .iter()
        .any(|marker| name.contains(marker));
    is_source && !is_test
}

/// The source files under `path`, or `path` itself when it's a file.
fn collect_files(path: &Path, files: &mut Vec<PathBuf>) -> Result<(), RunError> {
    if path.is_file() {
        files.push(path.to_owned());
        return Ok(());
    }
    let read_error = |source| RunError::Read {
        path: path.to_owned(),
        source,
    };
    for entry in fs::read_dir(path).map_err(read_error)? {
        let entry_path = entry.map_err(read_error)?.path();
        if entry_path.is_dir() {
            collect_files(&entry_path, files)?;
            continue;
        }
        if is_mutated(&entry_path) {
            files.push(entry_path);
        }
    }
    Ok(())
}

/// The path from the project's root, with `/` between folders.
fn relate(project: &Path, file: &Path) -> String {
    let relative = file.strip_prefix(project).unwrap_or(file);
    relative
        .components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

fn run(arguments: &Arguments) -> Result<usize, RunError> {
    let mut files = Vec::new();
    for path in &arguments.paths {
        collect_files(path, &mut files)?;
    }
    files.retain(|file| !arguments.excluded.contains(file));
    files.sort();
    let mut mutants = Vec::new();
    for file in &files {
        let source = fs::read_to_string(file).map_err(|source| RunError::Read {
            path: file.clone(),
            source,
        })?;
        let name = relate(&arguments.project, file);
        let generated =
            generate::generate(&name, &source).map_err(|source| RunError::Generate {
                file: name.clone(),
                source,
            })?;
        mutants.extend(generated);
    }
    let count = mutants.len();
    let json = serde_json::to_string_pretty(&MutantsFile::new(mutants))
        .expect("the mutants file has no maps with non-string keys");
    let write_error = |source| RunError::Write {
        path: arguments.out.clone(),
        source,
    };
    fs::create_dir_all(&arguments.out).map_err(write_error)?;
    fs::write(arguments.out.join("mutants.json"), json).map_err(write_error)?;
    Ok(count)
}

fn main() -> ExitCode {
    let result = parse_arguments(std::env::args().skip(1)).and_then(|arguments| run(&arguments));
    match result {
        Ok(count) => {
            // A closed stdout isn't worth failing over; the file is written.
            let _ignored = writeln!(io::stdout().lock(), "{count} mutants");
            ExitCode::SUCCESS
        }
        Err(error) => {
            let _ignored = writeln!(io::stderr().lock(), "{error}");
            ExitCode::FAILURE
        }
    }
}
