import { beforeEach, describe, expect, it, vi } from "vitest";
import { freshPinia } from "~/tests/pinia";
import { useSsoHandoffController } from "./SsoHandoff.controller";
import * as authApi from "~/services/auth.api";
import { ApiError } from "~/services/http.client";
import { browserSession } from "~/tests/browser-session";
import { mountController } from "~/tests/mount-controller";

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
  window.history.replaceState(null, "", "/login/sso");
});

describe("useSsoHandoffController", () => {
  it("trades the fragment code for a session and strips it from the URL", async () => {
    window.history.replaceState(null, "", "/login/sso#code=ssoh_abc");
    vi.mocked(authApi.exchangeSso).mockResolvedValueOnce({
      session: browserSession(),
      redirectTo: "/projects/p/api",
    });
    const { controller } = mountController(useSsoHandoffController);
    await settle();
    expect(window.location.hash).toBe("");
    expect(authApi.exchangeSso).toHaveBeenCalledWith("ssoh_abc");
    expect(routerReplace).toHaveBeenCalledWith("/projects/p/api");
    expect(controller.error.value).toBeNull();
  });

  it("falls back to projects when no safe redirect came back", async () => {
    window.history.replaceState(null, "", "/login/sso#code=ssoh_abc");
    vi.mocked(authApi.exchangeSso).mockResolvedValueOnce({
      session: browserSession(),
      redirectTo: null,
    });
    mountController(useSsoHandoffController);
    await settle();
    expect(routerReplace).toHaveBeenCalledWith({ name: "projects" });
  });

  it("explains a missing or expired code", async () => {
    const { controller } = mountController(useSsoHandoffController);
    await settle();
    expect(controller.error.value).toContain("incomplete");
    expect(authApi.exchangeSso).not.toHaveBeenCalled();

    window.history.replaceState(null, "", "/login/sso#code=ssoh_old");
    vi.mocked(authApi.exchangeSso).mockRejectedValueOnce(
      new ApiError(401, { SSO_HANDOFF_INVALID: "expired" }),
    );
    await controller.complete();
    expect(controller.error.value).toContain("expired");
    expect(routerReplace).not.toHaveBeenCalled();
  });
});
