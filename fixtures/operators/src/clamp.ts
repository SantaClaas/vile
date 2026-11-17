// Block removal and relational mutants. `>` turned into `>=` is missed and can't be caught: at
// value === maximum both return the same number. It's equivalent, but only reasoning shows it;
// the minified code differs, so it counts as missed.
export function clamp(value: number, maximum: number): number {
  if (value > maximum) {
    return maximum;
  }
  return value;
}
