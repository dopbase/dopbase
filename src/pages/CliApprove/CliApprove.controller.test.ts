import {
  afterEach,
  beforeEach,
  describe,
  expect,
  it,
  onTestFinished,
  vi,
} from "vitest";
import { createPinia, setActivePinia } from "pinia";
import {
  DEFAULT_SESSION_DAYS,
  displayCode,
  formatCountdown,
  normalizeCode,
  useCliApproveController,
} from "./CliApprove.controller";
import * as authApi from "~/services/auth.api";
import { ApiError } from "~/services/http.client";
import { useAuthStore } from "~/stores/auth.store";
import { useReauthentication } from "~/composable/reauthentication";
import { mountController } from "~/tests/mount-controller";
import { browserSessionInfo } from "~/tests/browser-session";

const { query, routerReplace } = vi.hoisted(() => ({
  query: { value: {} as Record<string, string> },
  routerReplace: vi.fn(),
}));

vi.mock("vue-router", () => ({
  useRoute: () => ({ query: query.value }),
  useRouter: () => ({ replace: routerReplace }),
}));

vi.mock("~/services/auth.api");

const pending: authApi.CliLoginRequest = {
  hostname: "dev-box",
  ipAddress: "203.0.113.7",
  createdAt: "2026-10-24T10:00:00Z",
  expiresAt: "2026-10-24T10:10:00Z",
  purpose: "login",
};

function settle(): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve));
}

beforeEach(() => {
  setActivePinia(createPinia());
  vi.clearAllMocks();
  query.value = {};
  // Signed in already, in another tab, before the CLI asked.
  vi.mocked(authApi.fetchSession).mockResolvedValue({
    ...browserSessionInfo(),
    email: "dev@example.com",
    role: "admin",
    csrfToken: "csrf_tab",
  });
});

afterEach(() => {
  vi.useRealTimers();
});

/** Approve a looked-up request with fake timers running. */
async function approveWithClock() {
  query.value = { code: "WDJB-MJHT" };
  vi.mocked(authApi.inspectCliLogin).mockResolvedValueOnce(pending);
  vi.mocked(authApi.approveCliLogin).mockResolvedValueOnce();
  const mounted = mountController(useCliApproveController);
  await settle();
  vi.useFakeTimers();
  const close = vi.spyOn(window, "close").mockImplementation(() => {});
  onTestFinished(() => close.mockRestore());
  await mounted.controller.approve();
  return { ...mounted, close };
}

describe("code helpers", () => {
  it("reads a code however it was typed", () => {
    expect(normalizeCode("wdjb-mjht")).toBe("WDJBMJHT");
    expect(normalizeCode(" WDJB MJHT ")).toBe("WDJBMJHT");
    // Vowels and digits are not in the alphabet.
    expect(normalizeCode("AEIO-1234")).toBe("");
    expect(normalizeCode(undefined)).toBe("");
    expect(displayCode("WDJBMJHT")).toBe("WDJB-MJHT");
  });
});

describe("useCliApproveController", () => {
  it("uses the browser's session and says who is signed in", async () => {
    query.value = { code: "wdjb-mjht" };
    vi.mocked(authApi.inspectCliLogin).mockResolvedValueOnce(pending);
    const { controller: c } = mountController(useCliApproveController);
    expect(c.step.value).toBe("checking");
    await settle();
    // The tab came from the terminal, so its CSRF token comes from here.
    expect(useAuthStore().csrfToken).toBe("csrf_tab");
    expect(authApi.inspectCliLogin).toHaveBeenCalledWith("WDJBMJHT");
    expect(authApi.logout).not.toHaveBeenCalled();
    expect(c.step.value).toBe("review");
    expect(c.request.value).toEqual(pending);
    expect(c.isConfirmation.value).toBe(false);
    expect(c.account.value).toEqual({
      email: "dev@example.com",
      role: "Administrator",
      initial: "D",
    });
  });

  it("asks for the whole code and explains a wrong one", async () => {
    const { controller: c } = mountController(useCliApproveController);
    await settle();
    expect(c.step.value).toBe("enter");
    c.code.value = "WDJB";
    await c.lookUp();
    expect(c.codeError.value).toContain("all 8 characters");
    expect(authApi.inspectCliLogin).not.toHaveBeenCalled();

    c.code.value = "BCDFGHJK";
    vi.mocked(authApi.inspectCliLogin).mockRejectedValueOnce(
      new ApiError(404, { DEVICE_CODE_INVALID: "invalid" }),
    );
    await c.lookUp();
    expect(c.step.value).toBe("enter");
    expect(c.codeError.value).toContain("wrong or has expired");

    vi.mocked(authApi.inspectCliLogin).mockRejectedValueOnce(
      new ApiError(429, { RATE_LIMITED: "slow down" }),
    );
    await c.lookUp();
    expect(c.codeError.value).toContain("Too many wrong codes");
  });

  it("sends someone who isn't signed in to sign in, then back", async () => {
    query.value = { code: "WDJBMJHT" };
    vi.mocked(authApi.fetchSession).mockRejectedValueOnce(
      new ApiError(401, { AUTHENTICATION_REQUIRED: "log in" }),
    );
    const auth = useAuthStore();
    mountController(useCliApproveController);
    await settle();
    // Asked quietly, so the 401 raises no "session expired" state.
    expect(authApi.fetchSession).toHaveBeenCalledWith({
      notifyAuthEvents: false,
    });
    expect(authApi.inspectCliLogin).not.toHaveBeenCalled();
    expect(authApi.logout).not.toHaveBeenCalled();
    expect(auth.sessionExpired).toBe(false);
    expect(routerReplace).toHaveBeenCalledWith({
      name: "login",
      query: {
        redirect: "/cli/approve?code=WDJB-MJHT",
        notice: "cli-approve",
      },
    });
  });

  it("signs in again when this tab still can't write, even if sign-out fails", async () => {
    query.value = { code: "WDJBMJHT" };
    vi.mocked(authApi.inspectCliLogin).mockRejectedValueOnce(
      new ApiError(403, { AUTHORIZATION_DENIED: "csrf" }),
    );
    vi.mocked(authApi.logout).mockRejectedValueOnce(
      new ApiError(403, { AUTHORIZATION_DENIED: "csrf" }),
    );
    mountController(useCliApproveController);
    await settle();
    expect(authApi.logout).toHaveBeenCalled();
    expect(routerReplace).toHaveBeenCalledWith(
      expect.objectContaining({ name: "login" }),
    );
  });

  it("switches account by signing out and coming back to the code", async () => {
    query.value = { code: "WDJBMJHT" };
    vi.mocked(authApi.inspectCliLogin).mockResolvedValueOnce(pending);
    vi.mocked(authApi.logout).mockResolvedValueOnce();
    const { controller: c } = mountController(useCliApproveController);
    await settle();
    await c.switchAccount();
    expect(authApi.logout).toHaveBeenCalled();
    expect(routerReplace).toHaveBeenCalledWith({
      name: "login",
      query: {
        redirect: "/cli/approve?code=WDJB-MJHT",
        notice: "cli-approve",
      },
    });
  });

  it("asks to confirm when the session is older, then approves", async () => {
    query.value = { code: "WDJB-MJHT" };
    vi.mocked(authApi.inspectCliLogin).mockResolvedValueOnce(pending);
    vi.mocked(authApi.approveCliLogin)
      .mockRejectedValueOnce(
        new ApiError(403, { RECENT_AUTHENTICATION_REQUIRED: "confirm" }),
      )
      .mockResolvedValueOnce();
    vi.mocked(authApi.reauthenticate).mockResolvedValueOnce();
    const { controller: c } = mountController(useCliApproveController);
    await settle();
    const reauth = useReauthentication();
    const approving = c.approve();
    await settle();
    expect(reauth.isOpen.value).toBe(true);
    expect(reauth.reason.value).toContain("90 days");
    expect(authApi.logout).not.toHaveBeenCalled();

    await reauth.submit("correct-horse-123");
    await approving;
    expect(authApi.approveCliLogin).toHaveBeenCalledTimes(2);
    expect(c.step.value).toBe("approved");
  });

  it("sends people to sign in when the session ends on the page", async () => {
    query.value = { code: "WDJB-MJHT" };
    vi.mocked(authApi.inspectCliLogin).mockResolvedValueOnce(pending);
    vi.mocked(authApi.approveCliLogin).mockRejectedValueOnce(
      new ApiError(401, { AUTHENTICATION_REQUIRED: "sign in" }),
    );
    const { controller: c } = mountController(useCliApproveController);
    await settle();
    const auth = useAuthStore();
    expect(auth.isAuthenticated).toBe(true);
    await c.approve();
    // Forgotten here as well, so the login page doesn't bounce to the
    // dashboard and its "session expired" dialog.
    expect(auth.isAuthenticated).toBe(false);
    expect(auth.sessionExpired).toBe(false);
    expect(authApi.logout).not.toHaveBeenCalled();
    expect(routerReplace).toHaveBeenCalledWith(
      expect.objectContaining({ name: "login" }),
    );
  });

  it("keeps the request when the confirm dialog is closed", async () => {
    query.value = { code: "WDJB-MJHT" };
    vi.mocked(authApi.inspectCliLogin).mockResolvedValueOnce(pending);
    vi.mocked(authApi.approveCliLogin).mockRejectedValueOnce(
      new ApiError(403, { RECENT_AUTHENTICATION_REQUIRED: "confirm" }),
    );
    const { controller: c } = mountController(useCliApproveController);
    await settle();
    const reauth = useReauthentication();
    const approving = c.approve();
    await settle();
    reauth.dismiss();
    await approving;
    expect(c.step.value).toBe("review");
    expect(c.formError.value).toBeNull();
    expect(c.deciding.value).toBeNull();
  });

  it("approves the reviewed request", async () => {
    query.value = { code: "WDJB-MJHT" };
    vi.mocked(authApi.inspectCliLogin).mockResolvedValueOnce(pending);
    vi.mocked(authApi.approveCliLogin).mockResolvedValueOnce();
    const { controller: c } = mountController(useCliApproveController);
    await settle();
    await c.approve();
    expect(authApi.approveCliLogin).toHaveBeenCalledWith("WDJBMJHT");
    expect(c.step.value).toBe("approved");
    expect(c.deciding.value).toBeNull();
  });

  it("denies the reviewed request and can start over", async () => {
    query.value = { code: "WDJB-MJHT" };
    vi.mocked(authApi.inspectCliLogin).mockResolvedValueOnce({
      ...pending,
      purpose: "reauth",
    });
    vi.mocked(authApi.denyCliLogin).mockResolvedValueOnce();
    const { controller: c } = mountController(useCliApproveController);
    await settle();
    expect(c.isConfirmation.value).toBe(true);
    await c.deny();
    expect(authApi.denyCliLogin).toHaveBeenCalledWith("WDJBMJHT");
    expect(c.step.value).toBe("denied");
    expect(c.closingIn.value).toBeNull();
    c.startOver();
    expect(c.step.value).toBe("enter");
    expect(c.code.value).toBe("");
    expect(c.request.value).toBeNull();
  });

  it("keeps the review open when the code expired meanwhile", async () => {
    query.value = { code: "WDJB-MJHT" };
    vi.mocked(authApi.inspectCliLogin).mockResolvedValueOnce(pending);
    vi.mocked(authApi.approveCliLogin).mockRejectedValueOnce(
      new ApiError(404, { DEVICE_CODE_INVALID: "expired" }),
    );
    const { controller: c } = mountController(useCliApproveController);
    await settle();
    await c.approve();
    expect(c.step.value).toBe("review");
    expect(c.formError.value).toContain("wrong or has expired");
  });

  it("counts down after approving, then closes the tab", async () => {
    const { controller: c, close } = await approveWithClock();
    expect(c.closingIn.value).toBe(5);
    vi.advanceTimersByTime(4000);
    expect(c.closingIn.value).toBe(1);
    expect(close).not.toHaveBeenCalled();
    vi.advanceTimersByTime(1000);
    expect(close).toHaveBeenCalledOnce();
    expect(c.closingIn.value).toBeNull();
    expect(c.closeBlocked.value).toBe(false);

    // Still here, as when the browser refuses to close a tab it opened.
    vi.advanceTimersByTime(300);
    expect(c.closeBlocked.value).toBe(true);
  });

  it("stops the countdown when asked, starting over, or leaving", async () => {
    const kept = await approveWithClock();
    kept.controller.keepOpen();
    expect(kept.controller.closingIn.value).toBeNull();
    vi.advanceTimersByTime(10_000);
    expect(kept.close).not.toHaveBeenCalled();
    expect(kept.controller.closeBlocked.value).toBe(false);
    vi.useRealTimers();

    const restarted = await approveWithClock();
    restarted.controller.startOver();
    vi.advanceTimersByTime(10_000);
    expect(restarted.close).not.toHaveBeenCalled();
    vi.useRealTimers();

    const left = await approveWithClock();
    left.unmount();
    vi.advanceTimersByTime(10_000);
    expect(left.close).not.toHaveBeenCalled();
  });

  it("names the client and how long the session lasts", async () => {
    query.value = { code: "WDJB-MJHT" };
    vi.mocked(authApi.inspectCliLogin).mockResolvedValueOnce({
      ...pending,
      client: "dopbase 0.2.0 on macOS",
      sessionDays: 30,
    });
    const { controller } = mountController(useCliApproveController);
    await settle();
    expect(controller.request.value?.client).toBe("dopbase 0.2.0 on macOS");
    expect(controller.machine.value).toBe("dev-box");
    expect(controller.sessionDays.value).toBe(30);
  });

  it("assumes the usual session length when an older server doesn't say", async () => {
    query.value = { code: "WDJB-MJHT" };
    vi.mocked(authApi.inspectCliLogin).mockResolvedValueOnce(pending);
    const { controller } = mountController(useCliApproveController);
    await settle();
    expect(controller.sessionDays.value).toBe(DEFAULT_SESSION_DAYS);
  });

  it("counts down to expiry and stops offering approval at zero", async () => {
    vi.useFakeTimers({ toFake: ["Date", "setInterval", "clearInterval"] });
    vi.setSystemTime(new Date("2026-10-24T10:09:57Z"));
    query.value = { code: "WDJB-MJHT" };
    vi.mocked(authApi.inspectCliLogin).mockResolvedValueOnce(pending);
    const { controller } = mountController(useCliApproveController);
    await vi.waitFor(() => expect(controller.step.value).toBe("review"));
    expect(controller.expiresIn.value).toBe("0:03");
    vi.advanceTimersByTime(3000);
    expect(controller.step.value).toBe("expired");
    expect(controller.secondsLeft.value).toBe(0);
  });

  it("counts the request's lifetime when this clock disagrees with the server", async () => {
    // The fixture's deadline is long past by this machine's clock, yet the
    // server just said the request is pending.
    vi.useFakeTimers({ toFake: ["Date", "setInterval", "clearInterval"] });
    vi.setSystemTime(new Date("2026-11-30T00:00:00Z"));
    query.value = { code: "WDJB-MJHT" };
    vi.mocked(authApi.inspectCliLogin).mockResolvedValueOnce(pending);
    const { controller } = mountController(useCliApproveController);
    await vi.waitFor(() => expect(controller.step.value).toBe("review"));
    expect(controller.expiresIn.value).toBe("10:00");
  });

  it("starting over clears the countdown", async () => {
    query.value = { code: "WDJB-MJHT" };
    vi.mocked(authApi.inspectCliLogin).mockResolvedValueOnce(pending);
    const { controller } = mountController(useCliApproveController);
    await settle();
    expect(controller.secondsLeft.value).not.toBeNull();
    controller.startOver();
    expect(controller.secondsLeft.value).toBeNull();
    expect(controller.step.value).toBe("enter");
  });
});

describe("formatCountdown", () => {
  it("shows minutes and two-digit seconds, never below zero", () => {
    expect(formatCountdown(272)).toBe("4:32");
    expect(formatCountdown(5)).toBe("0:05");
    expect(formatCountdown(-3)).toBe("0:00");
  });
});
