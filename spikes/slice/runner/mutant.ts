// Reads the mutants the Rust side wrote to mutants.json, in vile_core's `MutantsFile` shape.

/** A mutant, with the fields the runner needs. `start` and `end` are UTF-8 byte offsets. */
export type Mutant = {
  id: string;
  file: string;
  operator: string;
  original: string;
  start: number;
  end: number;
  replacement: string;
  line: number;
};

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

function readMutant(value: unknown): Mutant | undefined {
  if (!isRecord(value)) return undefined;
  const { id, file, operator, original, patch, range } = value;
  if (!isRecord(patch) || !isRecord(patch["span"])) return undefined;
  if (!isRecord(range) || !isRecord(range["start"])) return undefined;
  const { start, end } = patch["span"];
  const { replacement } = patch;
  const { line } = range["start"];
  const isValid =
    typeof id === "string" &&
    typeof file === "string" &&
    typeof operator === "string" &&
    typeof original === "string" &&
    typeof start === "number" &&
    typeof end === "number" &&
    typeof replacement === "string" &&
    typeof line === "number";
  if (!isValid) return undefined;
  return { id, file, operator, original, start, end, replacement, line };
}

/** The mutants in a mutants.json, or the reason it can't be read. */
export function parse(text: string): Mutant[] | "invalid-file" | "newer-version" {
  let data: unknown;
  try {
    data = JSON.parse(text);
  } catch {
    return "invalid-file";
  }
  if (!isRecord(data) || !Array.isArray(data["mutants"])) return "invalid-file";
  if (data["formatVersion"] !== 1) return "newer-version";
  const mutants = data["mutants"].map((value) => readMutant(value));
  return mutants.every((mutant) => mutant !== undefined) ? mutants : "invalid-file";
}

/**
 * The source with the mutant's patch applied. The span counts UTF-8 bytes, as Oxc's spans do, so
 * the patch goes on the bytes, not on the string's UTF-16 code units.
 */
export function apply(source: Buffer, mutant: Mutant): string {
  return Buffer.concat([
    source.subarray(0, mutant.start),
    Buffer.from(mutant.replacement),
    source.subarray(mutant.end),
  ]).toString("utf8");
}
