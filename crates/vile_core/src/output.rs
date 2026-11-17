//! The JSON files in `vile.out/`, which other programs read.
//!
//! See "A tool, not an app" in docs/guidelines/principles.md. Their shape is part of vile's interface: a field that's renamed
//! or removed is a breaking change, and so is a change in what a field means.

use serde::{Deserialize, Serialize};

use crate::{
    mutant::{Mutant, MutantId},
    outcome::{Outcome, Summary},
};

/// The version of the files' shape. It goes up when a field is renamed, removed or means something
/// else, so a program can refuse a file it doesn't understand. Adding a field doesn't change it.
pub const FORMAT_VERSION: u32 = 1;

/// `vile.out/mutants.json`: every mutant of the run, written before any test runs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MutantsFile {
    pub format_version: u32,
    pub mutants: Vec<Mutant>,
}

/// What happened to one mutant: its record in `outcomes.json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Record {
    pub id: MutantId,
    pub outcome: Outcome,
    /// How long its tests ran, in milliseconds. `None` when no test ran.
    pub duration_milliseconds: Option<u64>,
    /// The tests that failed, for a killed mutant, as Vitest names them.
    pub failed_tests: Vec<String>,
}

/// `vile.out/outcomes.json`: what happened to each mutant, rewritten as the run goes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OutcomesFile {
    pub format_version: u32,
    pub records: Vec<Record>,
    pub summary: Summary,
    pub score: Option<f64>,
}

impl MutantsFile {
    /// The file for these mutants, in the current format.
    #[must_use]
    pub const fn new(mutants: Vec<Mutant>) -> Self {
        Self {
            format_version: FORMAT_VERSION,
            mutants,
        }
    }
}

impl OutcomesFile {
    /// The file for these records, with their summary and score, in the current format.
    #[must_use]
    pub fn new(records: Vec<Record>) -> Self {
        let summary = Summary::count(records.iter().map(|record| record.outcome));
        let score = summary.score();
        Self {
            format_version: FORMAT_VERSION,
            records,
            summary,
            score,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{MutantsFile, OutcomesFile, Record};
    use crate::{
        mutant::{Identity, Mutant, MutantId},
        operator::Operator,
        outcome::Outcome,
        patch::Patch,
        span::{Location, Span},
    };

    fn mutant() -> Mutant {
        let scope = vec!["isFree".to_owned()];
        let id = MutantId::new(&Identity {
            file: "src/slot.ts",
            scope: &scope,
            original: "<",
            operator: Operator::Relational,
            replacement: "<=",
            occurrence: 0,
        });
        Mutant {
            id,
            file: "src/slot.ts".to_owned(),
            operator: Operator::Relational,
            original: "<".to_owned(),
            patch: Patch {
                span: Span { start: 40, end: 41 },
                replacement: "<=".to_owned(),
            },
            location: Location {
                line: 3,
                column: 12,
            },
        }
    }

    /// The JSON is an interface, so its exact shape is pinned here: a change to it shows up in
    /// this test, and needs a reason in review (and a new `FORMAT_VERSION` if it breaks readers).
    #[test]
    fn writes_the_mutants_file_in_its_documented_shape() {
        let json = serde_json::to_value(MutantsFile::new(vec![mutant()])).expect("it serializes");
        assert_eq!(
            json,
            serde_json::json!({
                "formatVersion": 1,
                "mutants": [{
                    "id": "src/slot.ts:isFree:relational:af63b14c8601a37b:08098b07b4c86ff2:0",
                    "file": "src/slot.ts",
                    "operator": "relational",
                    "original": "<",
                    "patch": { "span": { "start": 40, "end": 41 }, "replacement": "<=" },
                    "location": { "line": 3, "column": 12 }
                }]
            })
        );
    }

    #[test]
    fn writes_the_outcomes_file_with_its_summary_and_score() {
        let records = vec![
            Record {
                id: mutant().id,
                outcome: Outcome::Killed,
                duration_milliseconds: Some(12),
                failed_tests: vec!["slot > isFree > refuses a taken slot".to_owned()],
            },
            Record {
                id: mutant().id,
                outcome: Outcome::Survived,
                duration_milliseconds: Some(9),
                failed_tests: Vec::new(),
            },
        ];
        let json = serde_json::to_value(OutcomesFile::new(records)).expect("it serializes");
        assert_eq!(json["formatVersion"], 1);
        assert_eq!(json["summary"]["killed"], 1);
        assert_eq!(json["summary"]["noCoverage"], 0);
        assert_eq!(json["score"], 0.5);
        assert_eq!(json["records"][0]["durationMilliseconds"], 12);
        assert_eq!(
            json["records"][0]["failedTests"][0],
            "slot > isFree > refuses a taken slot"
        );
    }

    #[test]
    fn reads_back_what_it_writes() {
        let file = MutantsFile::new(vec![mutant()]);
        let json = serde_json::to_string(&file).expect("it serializes");
        let read: MutantsFile = serde_json::from_str(&json).expect("it reads back");
        assert_eq!(read, file);
    }
}
