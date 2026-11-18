// Block removal and relational mutants. `>` turned into `>=` survives and can't be killed: at
// value === maximum both return the same number. It's equivalent, but only reasoning shows it;
// the minified code differs, so it counts as survived.
export function clamp(value: number, maximum: number): number {
  if (value > maximum) {
    return maximum;
  }
  return value;
}
