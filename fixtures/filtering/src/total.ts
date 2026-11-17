// Mutants vile filters out before any test runs.
export function total(a: number, b: number): number {
  // Arid: the arguments of a console call are never mutated.
  console.log("adding", a + b);
  // Equivalent: the minifier removes this block, so a mutant inside it changes nothing.
  if (false) {
    return a - b;
  }
  return a + b;
}

export function isAdult(age: number): boolean {
  // vile-disable-next-line relational -- the boundary is tested where age is entered
  return age >= 18;
}

// No test calls this, so its mutants are not reached.
export function isBefore(a: number, b: number): boolean {
  return a < b;
}
