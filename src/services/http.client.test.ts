import { beforeEach, describe, expect, it, vi } from "vitest";
import {
  ApiError,
  apiRequest,
  onReauthenticationRequired,
  onUnauthorized,
  registerCsrfProvider,
} from "./http.client";

const fetchMock = vi.fn();

beforeEach(() => {
  fetchMock.mockReset();
  vi.stubGlobal("fetch", fetchMock);
  registerCsrfProvider(() => null);
});

function jsonResponse(body: unknown, status = 200): Response {
  return {
    ok: status >= 200 && status < 300,
    status,
    json: async () => body,
  } as unknown as Response;
}

describe("apiRequest", () => {
  it("returns the envelope message and data", async () => {
    fetchMock.mockResolvedValueOnce(
      jsonResponse({
        success: true,
        message: "PROJECTS_FETCHED",
        data: [{ id: "prj_1" }],
      }),
    );
    const result = await apiRequest<{ id: string }[]>("/api/v1/projects");
    expect(result.message).toBe("PROJECTS_FETCHED");
    expect(result.data).toEqual([{ id: "prj_1" }]);
  });

  it("sends a JSON body with the right content type", async () => {
    fetchMock.mockResolvedValueOnce(
      jsonResponse({ success: true, message: "OK", data: null }),
    );
    await apiRequest("/api/v1/projects", {
      method: "POST",
      body: { name: "app" },
    });
    const [, init] = fetchMock.mock.calls[0];
    expect(init.method).toBe("POST");
    expect(init.headers["Content-Type"]).toBe("application/json");
    expect(init.body).toBe(JSON.stringify({ name: "app" }));
  });

  it("injects the CSRF header on mutations when provided", async () => {
    registerCsrfProvider(() => "csrf_123");
    fetchMock.mockResolvedValue(
      jsonResponse({ success: true, message: "OK", data: null }),
    );
    await apiRequest("/x", { method: "POST" });
    expect(fetchMock.mock.calls[0][1].headers["X-Dopbase-CSRF"]).toBe(
      "csrf_123",
    );
    await apiRequest("/x", { method: "GET" });
    expect(
      fetchMock.mock.calls[1][1].headers["X-Dopbase-CSRF"],
    ).toBeUndefined();
    await apiRequest("/x", { method: "POST", anonymous: true });
    expect(
      fetchMock.mock.calls[2][1].headers["X-Dopbase-CSRF"],
    ).toBeUndefined();
  });

  it("throws ApiError carrying the error code map", async () => {
    fetchMock.mockResolvedValueOnce(
      jsonResponse(
        { success: false, error: { EMAIL_INVAILD: "Please use proper email" } },
        422,
      ),
    );
    const error: unknown = await apiRequest("/x").catch((cause) => cause);
    expect(error).toBeInstanceOf(ApiError);
    const apiError = error as ApiError;
    expect(apiError.status).toBe(422);
    expect(apiError.hasCode("EMAIL_INVAILD")).toBe(true);
    expect(apiError.message).toBe("Please use proper email");
  });

  it("notifies unauthorized listeners on 401", async () => {
    const spy = vi.fn();
    const off = onUnauthorized(spy);
    fetchMock.mockResolvedValueOnce(
      jsonResponse(
        { success: false, error: { AUTHENTICATION_INVALID: "expired" } },
        401,
      ),
    );
    await expect(apiRequest("/x")).rejects.toBeInstanceOf(ApiError);
    expect(spy).toHaveBeenCalledTimes(1);
    off();
    fetchMock.mockResolvedValueOnce(
      jsonResponse({ success: false, error: {} }, 401),
    );
    await expect(apiRequest("/x")).rejects.toBeInstanceOf(ApiError);
    expect(spy).toHaveBeenCalledTimes(1);
  });

  it("notifies reauthentication listeners on the 403 challenge", async () => {
    const spy = vi.fn();
    const off = onReauthenticationRequired(spy);
    fetchMock.mockResolvedValueOnce(
      jsonResponse(
        {
          success: false,
          error: { RECENT_AUTHENTICATION_REQUIRED: "confirm password" },
        },
        403,
      ),
    );
    await expect(apiRequest("/x")).rejects.toBeInstanceOf(ApiError);
    expect(spy).toHaveBeenCalledTimes(1);
    off();
  });

  it("does not treat other 403 codes as reauth challenges", async () => {
    const spy = vi.fn();
    const off = onReauthenticationRequired(spy);
    fetchMock.mockResolvedValueOnce(
      jsonResponse(
        { success: false, error: { AUTHORIZATION_DENIED: "no" } },
        403,
      ),
    );
    await expect(apiRequest("/x")).rejects.toBeInstanceOf(ApiError);
    expect(spy).not.toHaveBeenCalled();
    off();
  });

  it("maps network failures to a status-0 ApiError", async () => {
    fetchMock.mockRejectedValueOnce(new TypeError("failed to fetch"));
    const error: unknown = await apiRequest("/x").catch((cause) => cause);
    expect(error).toBeInstanceOf(ApiError);
    const apiError = error as ApiError;
    expect(apiError.status).toBe(0);
    expect(apiError.hasCode("NETWORK_ERROR")).toBe(true);
  });
});
