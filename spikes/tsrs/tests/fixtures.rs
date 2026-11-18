//! The acceptance check of #13: the fixtures' compile-error mutants get diagnostics from tsrs,
//! and their other mutants type-check.
#![expect(
    clippy::unwrap_used,
    reason = "tests may unwrap (docs/guidelines/rust.md)"
)]

use std::path::PathBuf;

use serde::Deserialize;
use vile_core::patch::Patch;
use vile_core::span::Span;
use vile_spike_tsrs::project::Project;

#[derive(Deserialize)]
struct Expected {
    mutants: Vec<ExpectedMutant>,
}

#[derive(Deserialize)]
struct ExpectedMutant {
    file: String,
    line: usize,
    original: Option<String>,
    replacement: String,
    outcome: String,
}

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures")
        .join(name)
        .canonicalize()
        .unwrap()
}

fn load(name: &str) -> (Project, PathBuf, Expected) {
    let directory = fixture(name);
    let config = directory.join("tsconfig.json");
    let project = Project::load(config.to_str().unwrap()).unwrap();
    let expected = std::fs::read_to_string(directory.join("expected.json")).unwrap();
    (project, directory, serde_json::from_str(&expected).unwrap())
}

fn line_start(source: &str, line: usize) -> usize {
    source
        .split_inclusive('\n')
        .take(line - 1)
        .map(str::len)
        .sum()
}

fn to_offset(offset: usize) -> u32 {
    u32::try_from(offset).unwrap()
}

/// The patch of a mutant, found the way fixtures/README.md writes them.
fn find_patch(project: &Project, file: &str, mutant: &ExpectedMutant) -> Patch {
    let source = project.text(file).unwrap();
    let start = line_start(source, mutant.line);
    // Body replacement has no original: its patch is the body of the function on the line.
    let Some(original) = &mutant.original else {
        let brace = start + source[start..].find('{').unwrap();
        return Patch {
            span: project.find_function(file, to_offset(brace)).unwrap().body,
            replacement: mutant.replacement.clone(),
        };
    };
    let offset = start + source[start..].find(original.as_str()).unwrap();
    let span = Span::new(to_offset(offset), to_offset(offset + original.len())).unwrap();
    Patch {
        span,
        replacement: mutant.replacement.clone(),
    }
}

fn check_fixture(name: &str) {
    let (project, directory, expected) = load(name);
    let mut wrong = Vec::new();
    for mutant in &expected.mutants {
        let file = directory.join(&mutant.file);
        let file = file.to_str().unwrap();
        let patch = find_patch(&project, file, mutant);
        let checked = project.check(file, &patch).unwrap();
        let is_compile_error = !checked.diagnostics.is_empty();
        if is_compile_error != (mutant.outcome == "compile-error") {
            wrong.push(format!(
                "{}:{} {:?}: {:?}",
                mutant.file, mutant.line, mutant.replacement, checked.diagnostics
            ));
        }
    }
    assert!(
        wrong.is_empty(),
        "mutants with the wrong check result: {wrong:#?}"
    );
}

#[test]
fn finds_the_compile_errors_in_the_operators_fixture() {
    check_fixture("operators");
}

#[test]
fn finds_the_compile_errors_in_the_body_replacement_fixture() {
    check_fixture("body-replacement");
}

#[test]
fn checks_unchanged_files_without_diagnostics() {
    for name in ["operators", "body-replacement"] {
        let (project, _, _) = load(name);
        let unchanged = Patch {
            span: Span::new(0, 0).unwrap(),
            replacement: String::new(),
        };
        for file in project.file_names() {
            let checked = project.check(&file, &unchanged).unwrap();
            assert_eq!(checked.diagnostics, [], "{file}");
            assert!(checked.is_reused, "{file}");
        }
    }
}

#[test]
fn reports_the_compile_error_where_it_is() {
    let (project, directory, _) = load("operators");
    let file = directory.join("src/name.ts");
    let file = file.to_str().unwrap();
    let source = project.text(file).unwrap();
    let start = source.find("nickname ?? name").unwrap();
    let patch = Patch {
        span: Span::new(
            to_offset(start),
            to_offset(start + "nickname ?? name".len()),
        )
        .unwrap(),
        replacement: "nickname".to_owned(),
    };
    let checked = project.check(file, &patch).unwrap();
    let [diagnostic] = checked.diagnostics.as_slice() else {
        panic!("one diagnostic, not {:?}", checked.diagnostics);
    };
    assert_eq!(diagnostic.code, 2322);
    // The span is in the patched text, on the `return` keyword, as tsc reports it.
    let patched = patch.apply(source).unwrap();
    assert_eq!(diagnostic.span.unwrap().text(&patched), Some("return"));
}

#[test]
fn finds_the_return_types_of_the_body_replacement_fixture() {
    let (project, directory, _) = load("body-replacement");
    let cases = [
        ("src/value.ts", "greet", "string"),
        ("src/value.ts", "evens", "number[]"),
        ("src/value.ts", "findName", "string | undefined"),
        ("src/value.ts", "record", "void"),
        ("src/countdown.ts", "isDone", "boolean"),
        ("src/countdown.ts", "countDown", "number"),
        ("src/cents.ts", "toCents", "Cents"),
    ];
    for (file, name, return_type) in cases {
        let file = directory.join(file);
        let file = file.to_str().unwrap();
        let source = project.text(file).unwrap();
        let declaration = source.find(&format!("function {name}(")).unwrap();
        let brace = declaration + source[declaration..].find('{').unwrap();
        let function = project.find_function(file, to_offset(brace)).unwrap();
        assert_eq!(function.return_type, return_type, "{name}");
        assert_eq!(function.body.start(), to_offset(brace), "{name}");
        assert!(function.body.text(source).unwrap().ends_with('}'), "{name}");
    }
}

#[test]
fn leaves_out_unused_declarations() {
    let directory = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("unused");
    std::fs::create_dir_all(&directory).unwrap();
    std::fs::write(
        directory.join("tsconfig.json"),
        r#"{ "compilerOptions": { "strict": true, "noUnusedLocals": true, "noUnusedParameters": true, "noEmit": true } }"#,
    )
    .unwrap();
    let source = "export function isEven(value: number): boolean {\n  return value % 2 === 0;\n}\n";
    std::fs::write(directory.join("even.ts"), source).unwrap();
    let directory = directory.canonicalize().unwrap();
    let project = Project::load(directory.join("tsconfig.json").to_str().unwrap()).unwrap();
    let file = directory.join("even.ts");
    let file = file.to_str().unwrap();
    let brace = to_offset(source.find('{').unwrap());
    let body = project.find_function(file, brace).unwrap().body;
    let empty = Patch {
        span: body,
        replacement: "{ return true; }".to_owned(),
    };
    assert_eq!(project.check(file, &empty).unwrap().diagnostics, []);
    let wrong = Patch {
        span: body,
        replacement: "{ return 0; }".to_owned(),
    };
    let codes: Vec<i32> = project
        .check(file, &wrong)
        .unwrap()
        .diagnostics
        .iter()
        .map(|diagnostic| diagnostic.code)
        .collect();
    assert_eq!(codes, [2322]);
}
