import { beforeEach, describe, expect, it, vi } from "vitest";
import { freshPinia } from "~/tests/pinia";
import { useLaunchController } from "./Launch.controller";
import * as authApi from "~/services/auth.api";
import { ApiError } from "~/services/http.client";
import { browserSession, browserSessionInfo } from "~/tests/browser-session";
import { mountController } from "~/tests/mount-controller";
import { useAuthStore } from "~/stores/auth.store";

const routerReplace = vi.hoisted(() => vi.fn());

vi.mock("vue-router", () => ({
  useRouter: () => ({ replace: routerReplace }),
}));

vi.mock("~/services/auth.api");

function settle(): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve));
}

beforeEach(() => {
  freshPinia();
  vi.clearAllMocks();
  window.history.replaceState(null, "", "/launch");
});

describe("useLaunchController", () => {
  it("signs in with the fragment and wipes it from the address bar", async () => {
    window.history.replaceState(
      null,
      "",
      "/launch#identity=root%40example.com&passcode=GJZG-FLMT",
    );
    vi.mocked(authApi.launch).mockResolvedValueOnce(
      browserSession({ email: "root@example.com" }),
    );
    vi.mocked(authApi.fetchSession).mockResolvedValue(browserSessionInfo());
    const { controller } = mountController(useLaunchController);
    await settle();
    expect(window.location.hash).toBe("");
    expect(authApi.launch).toHaveBeenCalledWith(
      "root@example.com",
      "GJZG-FLMT",
    );
    expect(useAuthStore().isAuthenticated).toBe(true);
    expect(routerReplace).toHaveBeenCalledWith({ name: "projects" });
    expect(controller.error.value).toBeNull();
  });

  it("keeps a plus sign in the email", async () => {
    window.history.replaceState(
      null,
      "",
      "/launch#identity=ops%2Broot%40example.com&passcode=GJZG-FLMT",
    );
    vi.mocked(authApi.launch).mockResolvedValueOnce(browserSession());
    vi.mocked(authApi.fetchSession).mockResolvedValue(browserSessionInfo());
    mountController(useLaunchController);
    await settle();
    expect(authApi.launch).toHaveBeenCalledWith(
      "ops+root@example.com",
      "GJZG-FLMT",
    );
  });

  it("sends a signed-out browser without a fragment to sign in", async () => {
    vi.mocked(authApi.fetchSession).mockRejectedValueOnce(
      new ApiError(401, { AUTHENTICATION_REQUIRED: "no" }),
    );
    mountController(useLaunchController);
    await settle();
    expect(authApi.launch).not.toHaveBeenCalled();
    expect(routerReplace).toHaveBeenCalledWith({ name: "login" });
  });

  it("sends a signed-in browser without a fragment to projects", async () => {
    vi.mocked(authApi.fetchSession).mockResolvedValueOnce(browserSessionInfo());
    mountController(useLaunchController);
    await settle();
    expect(authApi.launch).not.toHaveBeenCalled();
    expect(routerReplace).toHaveBeenCalledWith({ name: "projects" });
  });

  it("treats a fragment missing the passcode as no fragment", async () => {
    window.history.replaceState(
      null,
      "",
      "/launch#identity=root%40example.com",
    );
    vi.mocked(authApi.fetchSession).mockRejectedValueOnce(
      new ApiError(401, { AUTHENTICATION_REQUIRED: "no" }),
    );
    mountController(useLaunchController);
    await settle();
    expect(authApi.launch).not.toHaveBeenCalled();
    expect(window.location.hash).toBe("");
    expect(routerReplace).toHaveBeenCalledWith({ name: "login" });
  });

  it("ignores a passcode in the query string and strips it", async () => {
    window.history.replaceState(
      null,
      "",
      "/launch?identity=root%40example.com&passcode=GJZG-FLMT",
    );
    vi.mocked(authApi.fetchSession).mockRejectedValueOnce(
      new ApiError(401, { AUTHENTICATION_REQUIRED: "no" }),
    );
    mountController(useLaunchController);
    await settle();
    expect(authApi.launch).not.toHaveBeenCalled();
    expect(window.location.search).toBe("");
    expect(routerReplace).toHaveBeenCalledWith({ name: "login" });
  });

  it("explains a used or expired link and stays on the page", async () => {
    window.history.replaceState(
      null,
      "",
      "/launch#identity=root%40example.com&passcode=GJZG-FLMT",
    );
    vi.mocked(authApi.launch).mockRejectedValueOnce(
      new ApiError(401, { PASSCODE_INVALID: "expired" }),
    );
    const { controller } = mountController(useLaunchController);
    await settle();
    expect(controller.error.value).toContain("expired or was already used");
    expect(routerReplace).not.toHaveBeenCalled();
  });

  it("tries again when a new link is pasted into the same tab", async () => {
    window.history.replaceState(
      null,
      "",
      "/launch#identity=root%40example.com&passcode=GJZG-FLMT",
    );
    vi.mocked(authApi.launch).mockRejectedValueOnce(
      new ApiError(401, { PASSCODE_INVALID: "expired" }),
    );
    const { controller } = mountController(useLaunchController);
    await settle();
    expect(controller.error.value).not.toBeNull();

    vi.mocked(authApi.launch).mockResolvedValueOnce(browserSession());
    vi.mocked(authApi.fetchSession).mockResolvedValue(browserSessionInfo());
    window.history.replaceState(
      null,
      "",
      "/launch#identity=root%40example.com&passcode=KTVB-RMQD",
    );
    window.dispatchEvent(new HashChangeEvent("hashchange"));
    await settle();
    expect(authApi.launch).toHaveBeenLastCalledWith(
      "root@example.com",
      "KTVB-RMQD",
    );
    expect(controller.error.value).toBeNull();
    expect(routerReplace).toHaveBeenCalledWith({ name: "projects" });
  });

  it("says to wait when there were too many tries", async () => {
    window.history.replaceState(
      null,
      "",
      "/launch#identity=root%40example.com&passcode=GJZG-FLMT",
    );
    vi.mocked(authApi.launch).mockRejectedValueOnce(
      new ApiError(429, { RATE_LIMITED: "slow down" }),
    );
    const { controller } = mountController(useLaunchController);
    await settle();
    expect(controller.error.value).toContain("Wait a few minutes");
  });
});
