// Arithmetic mutants. The only test buys one item, so `*` turned into `/` survives.
export function totalPrice(unitPrice: number, quantity: number, discount: number): number {
  return unitPrice * quantity - discount;
}
