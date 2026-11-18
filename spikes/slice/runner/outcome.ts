// What happened to each mutant, written as vile_core's `OutcomesFile` (outcomes.json). The
// summary and score follow vile_core's `Summary`, since the runner can't call the Rust yet.

/** The outcomes the slice can give: it checks no types and filters nothing. */
export type Outcome = "killed" | "survived" | "timeout" | "runtime-error" | "no-coverage";

/** One mutant's record, as vile_core's `Record` writes it. */
export type Record = {
  id: string;
  outcome: Outcome;
  durationMilliseconds: number | null;
  failedTests: string[];
};

type Summary = {
  killed: number;
  survived: number;
  timeout: number;
  compileError: number;
  runtimeError: number;
  equivalent: number;
  noCoverage: number;
  ignored: number;
};

/** How many records ended in each outcome. */
export function summarize(records: readonly Record[]): Summary {
  const count = (outcome: Outcome): number =>
    records
      .values()
      .filter((record) => record.outcome === outcome)
      .reduce((total) => total + 1, 0);
  return {
    killed: count("killed"),
    survived: count("survived"),
    timeout: count("timeout"),
    compileError: 0,
    runtimeError: count("runtime-error"),
    equivalent: 0,
    noCoverage: count("no-coverage"),
    ignored: 0,
  };
}

/** Killed and timed-out mutants as a share of those a test decided, or null when none counts. */
export function score(summary: Summary): number | null {
  const detected = summary.killed + summary.timeout;
  const counted = detected + summary.survived + summary.noCoverage;
  return counted === 0 ? null : detected / counted;
}

/** The text of outcomes.json for these records. */
export function write(records: readonly Record[]): string {
  const summary = summarize(records);
  return JSON.stringify(
    { formatVersion: 1, records, summary, score: score(summary) },
    undefined,
    2,
  );
}
