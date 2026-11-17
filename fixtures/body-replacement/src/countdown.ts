// isDone returning false makes the loop in countDown endless, so that mutant times out.
export function isDone(remaining: number): boolean {
  return remaining <= 0;
}

export function countDown(start: number): number {
  let remaining = start;
  let steps = 0;
  while (!isDone(remaining)) {
    remaining -= 1;
    steps += 1;
  }
  return steps;
}
