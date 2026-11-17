// Function bodies replaced with values of their return type.
export function greet(name: string): string {
  return `Hello, ${name}`;
}

export function evens(values: readonly number[]): number[] {
  return values.filter((value) => value % 2 === 0);
}

export function findName(names: readonly string[], prefix: string): string | undefined {
  return names.find((name) => name.startsWith(prefix));
}

// No test looks at the log, so emptying the body is missed: the side effect goes unobserved.
export function record(log: string[], entry: string): void {
  log.push(entry);
}
