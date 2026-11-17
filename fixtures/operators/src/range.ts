// Relational and logical mutants. The tests check both boundaries but no value above the maximum,
// so `<` turned into `!=` is missed.
export function isInRange(value: number, minimum: number, maximum: number): boolean {
  return value >= minimum && value < maximum;
}
