// A branded return type: a plain number isn't a Cents, so every replacement is a compile error.
export type Cents = number & { readonly brand: "Cents" };

export function toCents(euros: number): Cents {
  return Math.round(euros * 100) as Cents;
}
