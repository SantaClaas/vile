//! What happened to each mutant, and the counts and score of a run.

use serde::{Deserialize, Serialize};

/// What happened to a mutant (see the glossary in docs/GLOSSARY.md).
///
/// The names are the statuses of Stryker's mutation-testing-elements report schema, the standard
/// words of the research and PIT, plus "equivalent" from the research. They're part of vile's
/// interface: they appear in the JSON, and the exit codes depend on them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Outcome {
    /// A test failed with the mutant in place.
    Killed,
    /// Every test that reaches the mutant passed.
    Survived,
    /// The tests ran past the time limit and were stopped.
    Timeout,
    /// The mutant doesn't type-check, so no test ran.
    CompileError,
    /// The mutant broke something other than a test: the module failed to load, or the test run
    /// crashed.
    RuntimeError,
    /// The mutant can't change what the program does: its minified code is the original's.
    Equivalent,
    /// No test runs the mutant's site.
    NoCoverage,
    /// A disable comment or an arid rule kept the site from being tested.
    Ignored,
}

/// How many mutants ended in each outcome.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Summary {
    pub killed: u32,
    pub survived: u32,
    pub timeout: u32,
    pub compile_error: u32,
    pub runtime_error: u32,
    pub equivalent: u32,
    pub no_coverage: u32,
    pub ignored: u32,
}

impl Summary {
    /// Counts the outcomes.
    #[must_use]
    pub fn count(outcomes: impl IntoIterator<Item = Outcome>) -> Self {
        outcomes.into_iter().fold(Self::default(), Self::add)
    }

    #[must_use]
    const fn add(mut self, outcome: Outcome) -> Self {
        let count = match outcome {
            Outcome::Killed => &mut self.killed,
            Outcome::Survived => &mut self.survived,
            Outcome::Timeout => &mut self.timeout,
            Outcome::CompileError => &mut self.compile_error,
            Outcome::RuntimeError => &mut self.runtime_error,
            Outcome::Equivalent => &mut self.equivalent,
            Outcome::NoCoverage => &mut self.no_coverage,
            Outcome::Ignored => &mut self.ignored,
        };
        *count = count.saturating_add(1);
        self
    }

    /// The mutation score, from 0 to 1: killed and timed-out mutants as a share of those that were
    /// killed, timed out, survived or had no coverage, the same definition as Stryker's. Compile
    /// errors, runtime errors, equivalent and ignored mutants don't count, since no test decided
    /// them. `None` when no mutant counts, rather than a score that means nothing.
    #[must_use]
    pub fn score(&self) -> Option<f64> {
        let detected = u64::from(self.killed) + u64::from(self.timeout);
        let counted = detected + u64::from(self.survived) + u64::from(self.no_coverage);
        if counted == 0 {
            return None;
        }
        #[expect(
            clippy::cast_precision_loss,
            reason = "both counts are below 2^34, far inside the 2^53 an f64 holds exactly"
        )]
        Some(detected as f64 / counted as f64)
    }
}

#[cfg(test)]
mod tests {
    use super::{Outcome, Summary};

    #[test]
    fn counts_each_outcome() {
        let summary = Summary::count([Outcome::Killed, Outcome::Killed, Outcome::NoCoverage]);
        assert_eq!(
            summary,
            Summary {
                killed: 2,
                no_coverage: 1,
                ..Summary::default()
            }
        );
    }

    #[test]
    fn scores_killed_and_timeout_against_survived_and_no_coverage() {
        let summary = Summary::count([
            Outcome::Killed,
            Outcome::Timeout,
            Outcome::Survived,
            Outcome::NoCoverage,
            Outcome::CompileError,
            Outcome::RuntimeError,
            Outcome::Equivalent,
            Outcome::Ignored,
        ]);
        assert_eq!(summary.score(), Some(0.5));
    }

    #[test]
    fn has_no_score_when_no_mutant_counts() {
        assert_eq!(Summary::default().score(), None);
        assert_eq!(
            Summary::count([Outcome::CompileError, Outcome::Ignored]).score(),
            None
        );
    }

    #[test]
    fn writes_outcomes_in_kebab_case() {
        for (outcome, name) in [
            (Outcome::Killed, "killed"),
            (Outcome::Survived, "survived"),
            (Outcome::Timeout, "timeout"),
            (Outcome::CompileError, "compile-error"),
            (Outcome::RuntimeError, "runtime-error"),
            (Outcome::Equivalent, "equivalent"),
            (Outcome::NoCoverage, "no-coverage"),
            (Outcome::Ignored, "ignored"),
        ] {
            let json = serde_json::to_string(&outcome).expect("an outcome serializes");
            assert_eq!(json, format!("\"{name}\""));
        }
    }
}
