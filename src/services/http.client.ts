/**
 * Thin typed wrapper around `fetch` for the Dopbase REST API.
 *
 * Responsibilities:
 * - Parses the typed success envelope (`{ success, message, data }`) and the
 *   error envelope (`{ success: false, error: { CODE: message } }`).
 * - Surfaces every failure as an {@link ApiError} whose `codes` map lets
 *   callers branch on stable error codes instead of English messages.
 * - Attaches the session CSRF header to mutating requests for browser
 *   sessions. The token is provided by the auth store via
 *   {@link registerCsrfProvider} to keep this module store-free.
 * - Notifies registered listeners on 401 (session expired) and on 403
 *   `RECENT_AUTHENTICATION_REQUIRED` (reveal/export need a fresh password).
 */

export interface ApiEnvelope<T> {
  success: boolean;
  message: string;
  data: T;
}

/** Stable error code → safe human-readable message, as sent by the server. */
export type ApiErrorCodeMap = Record<string, string>;

export class ApiError extends Error {
  readonly status: number;
  readonly codes: ApiErrorCodeMap;

  constructor(status: number, codes: ApiErrorCodeMap, message?: string) {
    super(message ?? Object.values(codes)[0] ?? "The request failed.");
    this.name = "ApiError";
    this.status = status;
    this.codes = codes;
  }

  /** The first (and usually only) error code, e.g. `"EMAIL_INVAILD"`. */
  get firstCode(): string | undefined {
    return Object.keys(this.codes)[0];
  }

  hasCode(code: string): boolean {
    return code in this.codes;
  }
}

const CSRF_HEADER = "X-Dopbase-CSRF";
const MUTATING_METHODS = new Set(["POST", "PUT", "PATCH", "DELETE"]);

let csrfProvider: () => string | null = () => null;

/**
 * Registers where the CSRF token comes from. Called once by the auth store;
 * mutating requests read the token lazily on every call.
 */
export function registerCsrfProvider(provider: () => string | null): void {
  csrfProvider = provider;
}

type Listener = () => void;

const unauthorizedListeners = new Set<Listener>();
const reauthListeners = new Set<Listener>();

/** Subscribes to 401 responses (expired/invalid session). Returns an off fn. */
export function onUnauthorized(listener: Listener): () => void {
  unauthorizedListeners.add(listener);
  return () => unauthorizedListeners.delete(listener);
}

/**
 * Subscribes to 403 `RECENT_AUTHENTICATION_REQUIRED` responses, emitted by
 * reveal/export when the last password confirmation is more than ten minutes
 * old. Returns an unsubscribe function.
 */
export function onReauthenticationRequired(listener: Listener): () => void {
  reauthListeners.add(listener);
  return () => reauthListeners.delete(listener);
}

function emit(listeners: Set<Listener>): void {
  for (const listener of listeners) listener();
}

export interface ApiRequestOptions {
  method?: "GET" | "POST" | "PUT" | "PATCH" | "DELETE";
  body?: unknown;
  signal?: AbortSignal;
  /**
   * Skips CSRF header injection. Required for the two pre-session mutations
   * (login and first-admin bootstrap) which run before any CSRF token exists.
   */
  anonymous?: boolean;
}

export interface ApiResult<T> {
  message: string;
  data: T;
}

async function parseErrorResponse(response: Response): Promise<ApiError> {
  try {
    const payload = (await response.json()) as {
      error?: ApiErrorCodeMap;
    };
    return new ApiError(
      response.status,
      payload.error ??
        ({ INTERNAL_ERROR: "The server reported an error." } as const),
    );
  } catch {
    return new ApiError(response.status, {
      INTERNAL_ERROR: "The server returned an unreadable error response.",
    });
  }
}

/**
 * Performs one API call and resolves with `{ message, data }` from the typed
 * envelope. Throws {@link ApiError} for every non-2xx response and for
 * network failures (status `0`).
 */
export async function apiRequest<T>(
  path: string,
  options: ApiRequestOptions = {},
): Promise<ApiResult<T>> {
  const method = options.method ?? "GET";
  const headers: Record<string, string> = { Accept: "application/json" };
  if (options.body !== undefined) {
    headers["Content-Type"] = "application/json";
  }
  if (MUTATING_METHODS.has(method) && !options.anonymous) {
    const csrf = csrfProvider();
    if (csrf) headers[CSRF_HEADER] = csrf;
  }

  let response: Response;
  try {
    response = await fetch(path, {
      method,
      headers,
      credentials: "same-origin",
      body:
        options.body === undefined ? undefined : JSON.stringify(options.body),
      signal: options.signal,
    });
  } catch (error) {
    // AbortSignal cancellations are rethrown so callers can ignore them.
    if (options.signal?.aborted) throw error;
    throw new ApiError(0, {
      NETWORK_ERROR: "Cannot reach the Dopbase server.",
    });
  }

  if (!response.ok) {
    const apiError = await parseErrorResponse(response);
    if (response.status === 401) emit(unauthorizedListeners);
    if (
      response.status === 403 &&
      apiError.hasCode("RECENT_AUTHENTICATION_REQUIRED")
    ) {
      emit(reauthListeners);
    }
    throw apiError;
  }

  const payload = (await response.json()) as ApiEnvelope<T>;
  return { message: payload.message, data: payload.data };
}
