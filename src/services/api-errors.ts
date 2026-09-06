import { ApiError } from "./http.client";

export function isAbortError(cause: unknown): boolean {
  return (typeof DOMException !== "undefined" && cause instanceof DOMException)
    ? cause.name === "AbortError"
    : cause instanceof Error && cause.name === "AbortError";
}

export function isNetworkError(cause: unknown): boolean {
  return cause instanceof ApiError && cause.status === 0;
}

/** Returns a safe, operation-specific fallback for an API failure. */
export function errorMessage(cause: unknown, fallback: string): string {
  if (isAbortError(cause)) return "";
  if (isNetworkError(cause)) return "Cannot reach the Dopbase server.";
  if (cause instanceof Error && cause.message) return cause.message;
  return fallback;
}

export function hasApiCode(cause: unknown, code: string): boolean {
  return cause instanceof ApiError && cause.hasCode(code);
}
