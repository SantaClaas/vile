// Runs a project's Vitest tests against each mutant in a mutants.json, in this one process, and
// writes outcomes.json next to it. See spikes/slice/REPORT.md.
//
//   node spikes/slice/runner/run.ts <project> <mutants.json> [--minimum-timeout <ms>]
//
// Each mutant's patch is served from memory by a Vite plugin; the user's files never change.

import { readFileSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { Writable } from "node:stream";
import {
  createVitest,
  type Reporter,
  type TestRunResult,
  type TestSpecification,
  type Vite,
  type Vitest,
} from "vitest/node";

import * as mutantFile from "./mutant.ts";
import * as outcomeFile from "./outcome.ts";
import type { Record } from "./outcome.ts";

/** The plan's timeout: five times the baseline, and at least this long. */
const TIMEOUT_FACTOR = 5;
const DEFAULT_MINIMUM_TIMEOUT = 20_000;

/**
 * Stops a run at its first failed test. Vitest's own `bail` cancels from the worker over IPC, and
 * that cancel can arrive after the run has ended and cancel the next mutant's run instead.
 */
function createBail(): { reporter: Reporter; settle: () => Promise<void> } {
  let vitest: Vitest | undefined;
  let cancelling: Promise<void> = Promise.resolve();
  const reporter: Reporter = {
    onInit: (instance) => {
      vitest = instance;
    },
    onTestCaseResult: (testCase) => {
      if (vitest === undefined || testCase.result().state !== "failed") return;
      cancelling = vitest.cancelCurrentRun("test-failure");
    },
  };
  return { reporter, settle: () => cancelling };
}

/** Serves a patched source in place of the file on disk while its mutant runs. */
function servePatched(patches: ReadonlyMap<string, string>): Vite.Plugin {
  return {
    name: "vile-spike-patched-source",
    enforce: "pre",
    load(id) {
      const [file = id] = id.split("?");
      return patches.get(file);
    },
  };
}

/** The files that import `file` directly, in every project's Vite environments. */
function listImporters(vitest: Vitest, file: string): string[] {
  return vitest.projects
    .values()
    .flatMap((project) => Object.values(project.vite.environments))
    .flatMap((environment) => environment.moduleGraph.getModulesByFile(file) ?? [])
    .flatMap((module) => module.importers)
    .flatMap((importer) => (importer.file === null ? [] : [importer.file]))
    .toArray();
}

/** The test files that import `file`, directly or through other modules, from Vite's graph. */
function findTestFiles(vitest: Vitest, file: string, testFiles: ReadonlySet<string>): Set<string> {
  const found = new Set<string>();
  const seen = new Set<string>();
  const queue = [file];
  for (const current of queue) {
    if (seen.has(current)) continue;
    seen.add(current);
    if (testFiles.has(current)) {
      found.add(current);
      continue;
    }
    queue.push(...listImporters(vitest, current));
  }
  return found;
}

/**
 * The tests' result, or "timeout" when they ran past `timeout`. A worker caught in a synchronous
 * loop never answers Vitest's first cancel, so a second one makes the pool kill its process.
 */
async function runWithTimeout(
  vitest: Vitest,
  specifications: TestSpecification[],
  timeout: number,
): Promise<TestRunResult | "timeout"> {
  const run = vitest.runTestSpecifications(specifications);
  let timer: NodeJS.Timeout | undefined;
  const expired = new Promise<"timeout">((resolveTimeout) => {
    timer = setTimeout(() => {
      resolveTimeout("timeout");
    }, timeout);
  });
  const first = await Promise.race([run, expired]);
  clearTimeout(timer);
  if (first !== "timeout") return first;
  const asking = vitest.cancelCurrentRun("keyboard-input");
  const forcing = vitest.cancelCurrentRun("keyboard-input");
  await Promise.all([asking, forcing, run.catch(() => "cancelled")]);
  return "timeout";
}

/** The outcome of one run. Its result lists every module run so far, so only `selected` count. */
function judge(
  result: TestRunResult | "timeout",
  selected: ReadonlySet<string>,
): Pick<Record, "outcome" | "failedTests"> {
  if (result === "timeout") return { outcome: "timeout", failedTests: [] };
  const modules = result.testModules.filter((module) => selected.has(module.moduleId));
  const failedTests = modules.flatMap((module) =>
    module.children
      .allTests("failed")
      .map((test) => test.fullName)
      .toArray(),
  );
  if (failedTests.length > 0) return { outcome: "killed", failedTests };
  const isBroken =
    result.unhandledErrors.length > 0 || modules.some((module) => module.errors().length > 0);
  return { outcome: isBroken ? "runtime-error" : "survived", failedTests: [] };
}

type Arguments = { project: string; mutantsPath: string; minimumTimeout: number };

function parseArguments(argumentList: readonly string[]): Arguments | "usage" {
  const [project, mutantsPath, flag, value] = argumentList;
  if (project === undefined || mutantsPath === undefined) return "usage";
  if (flag === undefined) {
    return { project, mutantsPath, minimumTimeout: DEFAULT_MINIMUM_TIMEOUT };
  }
  const minimumTimeout = Number(value);
  if (flag !== "--minimum-timeout" || !Number.isInteger(minimumTimeout)) return "usage";
  return { project, mutantsPath, minimumTimeout };
}

/** A stream that drops what's written, for Vitest's own output; VILE_SPIKE_VERBOSE keeps it. */
function createSink(): Writable {
  return new Writable({
    write: (_chunk, _encoding, callback) => {
      callback();
    },
  });
}

const formatSeconds = (milliseconds: number): string => (milliseconds / 1000).toFixed(1);

async function runMutants({ project, mutantsPath, minimumTimeout }: Arguments): Promise<string> {
  const mutants = mutantFile.parse(readFileSync(mutantsPath, "utf8"));
  if (typeof mutants === "string") return `${mutantsPath}: ${mutants}`;
  const root = resolve(project);
  const patches = new Map<string, string>();
  const isVerbose = process.env["VILE_SPIKE_VERBOSE"] === "1";
  const output = isVerbose ? {} : { stdout: createSink(), stderr: createSink() };
  const started = performance.now();
  const bail = createBail();
  const vitest = await createVitest(
    {
      root,
      watch: false,
      pool: "forks",
      silent: true,
      reporters: [bail.reporter],
      // The on-disk cache skips the plugin's load hook, and could keep a patched module.
      fsModuleCache: false,
      configLoader: "native",
    },
    { plugins: [servePatched(patches)] },
    output,
  );
  await vitest.standalone();

  const specifications = await vitest.globTestSpecifications();
  const baselineStarted = performance.now();
  const baseline = await vitest.runTestSpecifications(specifications);
  const baselineDuration = performance.now() - baselineStarted;
  if (!baseline.testModules.every((module) => module.ok())) {
    await vitest.close();
    return "The tests fail without any mutant; fix them first.";
  }
  const timeout = Math.max(minimumTimeout, TIMEOUT_FACTOR * baselineDuration);
  const testFiles = new Set(specifications.map((specification) => specification.moduleId));

  const records: Record[] = [];
  const sources = new Map<string, Buffer>();
  for (const mutant of mutants) {
    const file = join(root, mutant.file);
    const source = sources.get(file) ?? readFileSync(file);
    sources.set(file, source);
    const reaching = findTestFiles(vitest, file, testFiles);
    const selected = specifications.filter((specification) => reaching.has(specification.moduleId));
    if (selected.length === 0) {
      records.push({
        id: mutant.id,
        outcome: "no-coverage",
        durationMilliseconds: null,
        failedTests: [],
      });
      continue;
    }
    patches.set(file, mutantFile.apply(source, mutant));
    vitest.invalidateFile(file);
    const mutantStarted = performance.now();
    // One mutant at a time: the patch is global to the Vite server.
    // oxlint-disable-next-line no-await-in-loop -- mutants share the server, so they run in turn
    const result = await runWithTimeout(vitest, selected, timeout);
    // oxlint-disable-next-line no-await-in-loop -- the next run may only start once this one stopped
    await bail.settle();
    const durationMilliseconds = Math.round(performance.now() - mutantStarted);
    patches.delete(file);
    vitest.invalidateFile(file);
    records.push({ id: mutant.id, durationMilliseconds, ...judge(result, reaching) });
    if (isVerbose) console.log(mutant.id, records.at(-1)?.outcome);
  }
  await vitest.close();

  writeFileSync(join(dirname(mutantsPath), "outcomes.json"), outcomeFile.write(records));
  const summary = outcomeFile.summarize(records);
  return [
    `${mutants.length.toString()} mutants in ${formatSeconds(performance.now() - started)} s`,
    `(baseline ${formatSeconds(baselineDuration)} s, timeout ${formatSeconds(timeout)} s)`,
    `killed ${summary.killed.toString()}, survived ${summary.survived.toString()},`,
    `timeout ${summary.timeout.toString()}, runtime error ${summary.runtimeError.toString()},`,
    `no coverage ${summary.noCoverage.toString()}`,
  ].join(" ");
}

const USAGE =
  "usage: node spikes/slice/runner/run.ts <project> <mutants.json> [--minimum-timeout <ms>]";

async function main(): Promise<number> {
  const parsed = parseArguments(process.argv.slice(2));
  if (parsed !== "usage") {
    console.log(await runMutants(parsed));
    return 0;
  }
  console.error(USAGE);
  return 2;
}

if (import.meta.main) process.exitCode = await main();
