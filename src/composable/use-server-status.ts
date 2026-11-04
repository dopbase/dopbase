/**
 * The server this page belongs to, for the auth screens' header, so people
 * can check they are on the instance they expect. Safe to show before any
 * authentication.
 */
export function useServerStatus() {
  return { endpoint: window.location.origin };
}
