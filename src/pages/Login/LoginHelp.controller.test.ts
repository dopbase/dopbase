import { describe, expect, it, vi } from "vitest";
import { useLoginHelpController } from "./LoginHelp.controller";
import * as authApi from "~/services/auth.api";
import { mountController } from "~/tests/mount-controller";
import { settle } from "~/tests/people";

vi.mock("~/services/auth.api");

describe("useLoginHelpController", () => {
  it("names the server and the single sign-on provider", async () => {
    vi.mocked(authApi.fetchAuthMethods).mockResolvedValueOnce({
      password: true,
      passwordForm: true,
      sso: { name: "Google Workspace" },
      cliBrowser: true,
    });
    const { controller } = mountController(useLoginHelpController);
    await settle();
    expect(controller.host).toBe(window.location.host);
    expect(controller.ssoName.value).toBe("Google Workspace");
  });

  it("still helps when the server can't say how people sign in", async () => {
    vi.mocked(authApi.fetchAuthMethods).mockRejectedValueOnce(new Error(""));
    const { controller } = mountController(useLoginHelpController);
    await settle();
    expect(controller.ssoName.value).toBeNull();
  });
});
