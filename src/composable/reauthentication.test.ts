import { beforeEach, expect, it, vi } from "vitest";
import { freshPinia } from "~/tests/pinia";
import { ApiError } from "~/services/http.client";
import * as authApi from "~/services/auth.api";
import { useReauthentication } from "./reauthentication";

vi.mock("~/services/auth.api");

beforeEach(() => {
  freshPinia();
  vi.mocked(authApi.reauthenticate).mockReset();
  useReauthentication().dismiss();
});

function challenge(): ApiError {
  return new ApiError(403, {
    RECENT_AUTHENTICATION_REQUIRED: "Confirm the password.",
  });
}

it("queues multiple challenges and completes every original promise", async () => {
  const reauth = useReauthentication();
  const first = vi
    .fn()
    .mockRejectedValueOnce(challenge())
    .mockResolvedValueOnce(undefined);
  const second = vi
    .fn()
    .mockRejectedValueOnce(challenge())
    .mockResolvedValueOnce(undefined);
  const firstResult = reauth.runWithReauth(first);
  const secondResult = reauth.runWithReauth(second);
  await vi.waitFor(() => {
    expect(first).toHaveBeenCalledTimes(1);
    expect(second).toHaveBeenCalledTimes(1);
  });
  vi.mocked(authApi.reauthenticate).mockResolvedValueOnce();
  await expect(reauth.submit("correct-password")).resolves.toBe(true);
  await expect(Promise.all([firstResult, secondResult])).resolves.toEqual([
    undefined,
    undefined,
  ]);
  expect(first).toHaveBeenCalledTimes(2);
  expect(second).toHaveBeenCalledTimes(2);
});

it("returns retry failures to the operation without calling them password errors", async () => {
  const reauth = useReauthentication();
  const failure = new Error("export network failure");
  const action = vi
    .fn()
    .mockRejectedValueOnce(challenge())
    .mockRejectedValueOnce(failure);
  const result = reauth.runWithReauth(action);
  const rejected = expect(result).rejects.toBe(failure);
  await vi.waitFor(() => expect(action).toHaveBeenCalledTimes(1));
  vi.mocked(authApi.reauthenticate).mockResolvedValueOnce();
  await expect(reauth.submit("correct-password")).resolves.toBe(true);
  await rejected;
  expect(reauth.error.value).toBeNull();
});

it("hands a session that ended back to the waiting action", async () => {
  const reauth = useReauthentication();
  const action = vi.fn().mockRejectedValueOnce(challenge());
  const result = reauth.runWithReauth(action);
  const signedOut = new ApiError(401, { AUTHENTICATION_REQUIRED: "sign in" });
  const rejected = expect(result).rejects.toBe(signedOut);
  await vi.waitFor(() => expect(action).toHaveBeenCalledTimes(1));
  vi.mocked(authApi.reauthenticate).mockRejectedValueOnce(signedOut);
  await expect(reauth.submit("correct-password")).resolves.toBe(false);
  await rejected;
  // Not a wrong password, and nothing left to confirm.
  expect(reauth.error.value).toBeNull();
  expect(reauth.isOpen.value).toBe(false);
  expect(action).toHaveBeenCalledTimes(1);
});

it("does not queue a challenge that arrives after cancellation", async () => {
  const reauth = useReauthentication();
  const scope = new AbortController();
  let reject!: (cause: unknown) => void;
  const action = vi.fn(
    () =>
      new Promise<void>((_resolve, fail) => {
        reject = fail;
      }),
  );
  const result = reauth.runWithReauth(action, scope.signal);
  const rejected = expect(result).rejects.toMatchObject({ name: "AbortError" });
  scope.abort();
  reject(challenge());
  await rejected;
  vi.mocked(authApi.reauthenticate).mockResolvedValueOnce();
  await reauth.submit("fixture-password");
  expect(action).toHaveBeenCalledTimes(1);
});

it("reports a blocked popup without starting single sign-on", async () => {
  const reauth = useReauthentication();
  const open = vi.spyOn(window, "open").mockReturnValueOnce(null);
  await expect(reauth.confirmWithSso()).resolves.toBe(false);
  expect(reauth.error.value).toContain("Allow pop-ups");
  expect(authApi.startSsoReauth).not.toHaveBeenCalled();
  open.mockRestore();
});

it("resumes parked work once the popup confirms the session", async () => {
  vi.useFakeTimers();
  try {
    const reauth = useReauthentication();
    const popup = { closed: false, close: vi.fn(), location: { href: "" } };
    const open = vi
      .spyOn(window, "open")
      .mockReturnValueOnce(popup as unknown as Window);
    vi.mocked(authApi.startSsoReauth).mockResolvedValueOnce(
      "https://idp.example.com/authorize?max_age=0",
    );
    const session = (recentAuthentication: boolean) => ({
      adminId: "usr_1",
      email: "a@b.c",
      sessionKind: "browser" as const,
      recentAuthentication,
      role: "member" as const,
    });
    vi.mocked(authApi.fetchSession)
      .mockResolvedValueOnce(session(false))
      .mockResolvedValueOnce(session(true));
    const action = vi
      .fn()
      .mockRejectedValueOnce(challenge())
      .mockResolvedValueOnce(undefined);
    const result = reauth.runWithReauth(action);
    await vi.waitFor(() => expect(action).toHaveBeenCalledTimes(1));

    const confirmed = reauth.confirmWithSso();
    await vi.advanceTimersByTimeAsync(2500);
    await expect(confirmed).resolves.toBe(true);
    await expect(result).resolves.toBeUndefined();
    expect(popup.location.href).toContain("max_age=0");
    expect(popup.close).toHaveBeenCalled();
    expect(action).toHaveBeenCalledTimes(2);
    open.mockRestore();
  } finally {
    vi.useRealTimers();
  }
});

it("stops waiting when the popup closes unconfirmed", async () => {
  vi.useFakeTimers();
  try {
    const reauth = useReauthentication();
    const popup = { closed: true, close: vi.fn(), location: { href: "" } };
    const open = vi
      .spyOn(window, "open")
      .mockReturnValueOnce(popup as unknown as Window);
    vi.mocked(authApi.startSsoReauth).mockResolvedValueOnce("https://idp");
    vi.mocked(authApi.fetchSession).mockResolvedValueOnce({
      adminId: "usr_1",
      email: "a@b.c",
      sessionKind: "browser",
      recentAuthentication: false,
    });
    const confirmed = reauth.confirmWithSso();
    await vi.advanceTimersByTimeAsync(1500);
    await expect(confirmed).resolves.toBe(false);
    expect(reauth.error.value).toContain("closed");
    open.mockRestore();
  } finally {
    vi.useRealTimers();
  }
});
