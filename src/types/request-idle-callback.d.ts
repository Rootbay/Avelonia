export {};

declare global {
  interface IdleRequestDeadline {
    didTimeout: boolean;
    timeRemaining(): number;
  }

  type IdleRequestCallback = (deadline: IdleRequestDeadline) => void;

  interface IdleRequestOptions {
    timeout?: number;
  }

  interface Window {
    requestIdleCallback?: (callback: IdleRequestCallback, options?: IdleRequestOptions) => number;
    cancelIdleCallback?: (handle: number) => void;
  }
}
