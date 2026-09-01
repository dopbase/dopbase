import { beforeEach, expect, it, vi } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import { ApiError } from "~/services/http.client";
import * as authApi from "~/services/auth.api";
import { useReauthentication } from "./reauthentication";

vi.mock("~/services/auth.api");

beforeEach(() => {
  setActivePinia(createPinia());
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
