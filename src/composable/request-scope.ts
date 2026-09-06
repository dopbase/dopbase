import { onUnmounted } from "vue";

/**
 * Owns the AbortController for the latest read started by a screen.
 * Consumers still decide how to store data and map failures.
 */
export function useRequestScope() {
  let current: AbortController | null = null;

  function begin(): AbortSignal {
    current?.abort();
    current = new AbortController();
    return current.signal;
  }

  function isCurrent(signal: AbortSignal): boolean {
    return current?.signal === signal && !signal.aborted;
  }

  function cancel(): void {
    current?.abort();
    current = null;
  }

  onUnmounted(cancel);

  return { begin, isCurrent, cancel };
}
