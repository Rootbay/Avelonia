declare module 'bun:test' {
  export function describe(name: string, fn: () => void | Promise<void>): void;
  export const it: typeof describe;
  export function beforeEach(fn: () => void | Promise<void>): void;
  export const mock: {
    fn<T extends (...args: unknown[]) => unknown>(impl?: T): T;
    module(id: string, factory: () => Record<string, unknown>): void;
  };
  type Matchers<T> = {
    toBe(expected: unknown): void;
    toEqual(expected: unknown): void;
    toBeDefined(): void;
    toBeUndefined(): void;
    toBeTruthy(): void;
    toBeFalsy(): void;
    toContain(item: unknown): void;
    toHaveLength(length: number): void;
    toBeLessThan(value: number): void;
    toBeLessThanOrEqual(value: number): void;
    toBeGreaterThan(value: number): void;
    toBeGreaterThanOrEqual(value: number): void;
    toThrow(error?: unknown): void;
    toMatchObject(expected: Partial<T>): void;
    not: Matchers<T>;
  };
  export function expect<T = unknown>(actual: T): Matchers<T>;
}
