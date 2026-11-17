//! What happened to each mutant, and the counts and score of a run.

use serde::{Deserialize, Serialize};

/// What happened to a mutant (see the glossary in docs/GLOSSARY.md). The names are part of vile's
/// interface: they appear in the JSON and the exit codes depend on them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Outcome {
    /// A test failed with the mutant in place.
    Caught,
    /// Every test that reaches the mutant passed.
    Missed,
    /// The tests ran past the time limit and were stopped.
    Timeout,
    /// The mutant doesn't type-check, so no test ran.
    Unviable,
    /// The mutant can't change what the program does: its minified code is the original's.
    Equivalent,
    /// No test runs the mutant's site.
    NotReached,
    /// A disable comment or an arid rule kept the site from being mutated.
    Skipped,
}

/// How many mutants ended in each outcome.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Summary {
    pub caught: u32,
    pub missed: u32,
    pub timeout: u32,
    pub unviable: u32,
    pub equivalent: u32,
    pub not_reached: u32,
    pub skipped: u32,
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
            Outcome::Caught => &mut self.caught,
            Outcome::Missed => &mut self.missed,
            Outcome::Timeout => &mut self.timeout,
            Outcome::Unviable => &mut self.unviable,
            Outcome::Equivalent => &mut self.equivalent,
            Outcome::NotReached => &mut self.not_reached,
            Outcome::Skipped => &mut self.skipped,
        };
        *count = count.saturating_add(1);
        self
    }

    /// The mutation score, from 0 to 1: caught and timed-out mutants as a share of those that were
    /// caught, timed out, missed or not reached, the same definition as Stryker's. Unviable,
    /// equivalent and skipped mutants don't count, since no test could have caught them. `None`
    /// when no mutant counts, rather than a score that means nothing.
    #[must_use]
    pub fn score(&self) -> Option<f64> {
        let detected = u64::from(self.caught) + u64::from(self.timeout);
        let counted = detected + u64::from(self.missed) + u64::from(self.not_reached);
        if counted == 0 {
            return None;
        }
        #[expect(
            clippy::cast_precision_loss,
            reason = "both counts are below 2^33, far inside the 2^53 an f64 holds exactly"
        )]
        Some(detected as f64 / counted as f64)
    }
}

#[cfg(test)]
mod tests {
    use super::{Outcome, Summary};

    #[test]
    fn counts_each_outcome() {
        let summary = Summary::count([Outcome::Caught, Outcome::Caught, Outcome::NotReached]);
        assert_eq!(
            summary,
            Summary {
                caught: 2,
                not_reached: 1,
                ..Summary::default()
            }
        );
    }

    #[test]
    fn scores_caught_and_timeout_against_missed_and_not_reached() {
        let summary = Summary::count([
            Outcome::Caught,
            Outcome::Timeout,
            Outcome::Missed,
            Outcome::NotReached,
            Outcome::Unviable,
            Outcome::Equivalent,
            Outcome::Skipped,
        ]);
        assert_eq!(summary.score(), Some(0.5));
    }

    #[test]
    fn has_no_score_when_no_mutant_counts() {
        assert_eq!(Summary::default().score(), None);
        assert_eq!(
            Summary::count([Outcome::Unviable, Outcome::Skipped]).score(),
            None
        );
    }

    #[test]
    fn writes_outcomes_in_kebab_case() {
        let json = serde_json::to_string(&Outcome::NotReached).expect("an outcome serializes");
        assert_eq!(json, "\"not-reached\"");
    }
}
