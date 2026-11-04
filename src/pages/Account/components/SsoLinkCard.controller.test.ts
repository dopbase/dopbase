import { beforeEach, describe, expect, it, vi } from "vitest";
import { forgetQueries, freshPinia } from "~/tests/pinia";
import { useSsoLinkCardController } from "./SsoLinkCard.controller";
import * as authApi from "~/services/auth.api";
import { ApiError } from "~/services/http.client";
import { mountController } from "~/tests/mount-controller";

const { query, routerReplace } = vi.hoisted(() => ({
  query: { value: {} as Record<string, string> },
  routerReplace: vi.fn(),
}));

vi.mock("vue-router", () => ({
  useRoute: () => ({ query: query.value }),
  useRouter: () => ({ replace: routerReplace }),
}));

vi.mock("~/services/auth.api");

const unlinked: authApi.SsoIdentity = {
  available: true,
  providerName: "Acme SSO",
  linked: false,
  emailAtLink: null,
  linkedAt: null,
};

function settle(): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve));
}

beforeEach(() => {
  freshPinia();
  vi.clearAllMocks();
  query.value = {};
});

describe("useSsoLinkCardController", () => {
  it("stays hidden when single sign-on is off and nothing is linked", async () => {
    vi.mocked(authApi.fetchSsoIdentity).mockResolvedValueOnce({
      ...unlinked,
      available: false,
    });
    const { controller } = mountController(useSsoLinkCardController);
    await settle();
    expect(controller.visible.value).toBe(false);
  });

  it("reports the callback result once and clears it from the URL", async () => {
    query.value = { sso: "linked" };
    vi.mocked(authApi.fetchSsoIdentity).mockResolvedValueOnce({
      ...unlinked,
      linked: true,
    });
    const { controller } = mountController(useSsoLinkCardController);
    await settle();
    expect(controller.notice.value).toContain("linked");
    expect(routerReplace).toHaveBeenCalledWith({ query: {} });

    query.value = { sso_error: "identity_taken" };
    vi.mocked(authApi.fetchSsoIdentity).mockResolvedValueOnce(unlinked);
    forgetQueries();
    const second = mountController(useSsoLinkCardController).controller;
    await settle();
    expect(second.error.value).toContain("another Dopbase account");
  });

  it("explains a link that cannot start", async () => {
    vi.mocked(authApi.fetchSsoIdentity).mockResolvedValueOnce(unlinked);
    vi.mocked(authApi.startSsoLink).mockRejectedValueOnce(
      new ApiError(404, {
        SSO_NOT_CONFIGURED: "Single sign-on is not enabled.",
      }),
    );
    const { controller } = mountController(useSsoLinkCardController);
    await settle();
    expect(controller.visible.value).toBe(true);
    await controller.link();
    expect(controller.error.value).toBe("Single sign-on is not enabled.");
    expect(controller.linking.value).toBe(false);
  });

  it("unlinks only when a password remains to sign in with", async () => {
    const { useAuthStore } = await import("~/stores/auth.store");
    vi.mocked(authApi.fetchSsoIdentity).mockResolvedValue({
      ...unlinked,
      linked: true,
    });
    const { controller } = mountController(useSsoLinkCardController);
    await settle();
    expect(controller.canUnlink.value).toBe(true);

    useAuthStore().session = {
      adminId: "usr_1",
      email: "a@b.c",
      sessionKind: "browser",
      recentAuthentication: true,
      role: "member",
      hasPassword: false,
    };
    expect(controller.canUnlink.value).toBe(false);

    useAuthStore().session = {
      adminId: "usr_1",
      email: "a@b.c",
      sessionKind: "browser",
      recentAuthentication: true,
      role: "member",
      hasPassword: true,
    };
    vi.mocked(authApi.unlinkSso).mockResolvedValueOnce();
    await controller.unlink();
    expect(authApi.unlinkSso).toHaveBeenCalled();
    expect(controller.notice.value).toContain("unlinked");
  });
});
