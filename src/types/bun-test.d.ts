declare module 'bun:test' {
  export function describe(name: string, fn: () => void | Promise<void>): void;
  export const it: typeof describe;
  export function expect(actual: unknown): any;
}
