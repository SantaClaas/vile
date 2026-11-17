// A unary mutant that's caught, and a logical one that doesn't type-check: without `?? name`, the
// function could return undefined where it promises a string.
export function isVisible(isHidden: boolean): boolean {
  return !isHidden;
}

export function displayName(nickname: string | undefined, name: string): string {
  return nickname ?? name;
}
