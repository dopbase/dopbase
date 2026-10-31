import { beforeEach, describe, expect, it, vi } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import { useAccountActions } from "./account-actions";
import * as usersApi from "~/services/users.api";
import { ApiError } from "~/services/http.client";
import { useReauthentication } from "~/composable/reauthentication";
import { maya, settle, userFixture } from "~/tests/people";

vi.mock("~/services/users.api");

const challenge = () =>
  new ApiError(403, {
    RECENT_AUTHENTICATION_REQUIRED: "Confirm your password first.",
  });

beforeEach(() => {
  setActivePinia(createPinia());
  vi.clearAllMocks();
  vi.mocked(usersApi.disableUser).mockResolvedValue(
    userFixture({ status: "disabled" }),
  );
  vi.mocked(usersApi.enableUser).mockResolvedValue(maya);
  vi.mocked(usersApi.unlinkUserSso).mockResolvedValue(maya);
  vi.mocked(usersApi.deleteUser).mockResolvedValue();
});

describe("useAccountActions", () => {
  it("asks before disabling and does nothing until confirmed", () => {
    const done = vi.fn();
    const actions = useAccountActions(done);

    actions.ask("disable", maya);

    expect(actions.pending.value).toEqual({ kind: "disable", user: maya });
    expect(actions.copy.value).toMatchObject({
      title: "Disable maya@example.com?",
      confirmLabel: "Disable account",
      tone: "danger",
      confirmWord: undefined,
    });
    expect(actions.copy.value?.description).toContain(
      "you can turn the account back on",
    );
    expect(usersApi.disableUser).not.toHaveBeenCalled();
    expect(done).not.toHaveBeenCalled();
  });

  it("disables the account once confirmed and hands the action back", async () => {
    const done = vi.fn();
    const actions = useAccountActions(done);

    actions.ask("disable", maya);
    await actions.confirm();

    expect(usersApi.disableUser).toHaveBeenCalledWith("usr_maya");
    expect(actions.pending.value).toBeNull();
    expect(actions.copy.value).toBeNull();
    expect(actions.busy.value).toBe(false);
    expect(done).toHaveBeenCalledWith({ kind: "disable", user: maya });
  });

  it("turns a disabled user back on with a plain, non-danger confirmation", async () => {
    const done = vi.fn();
    const actions = useAccountActions(done);
    const disabled = userFixture({ status: "disabled" });

    actions.ask("enable", disabled);
    expect(actions.copy.value).toMatchObject({
      title: "Turn maya@example.com back on?",
      confirmLabel: "Turn back on",
      tone: "primary",
    });
    await actions.confirm();

    expect(usersApi.enableUser).toHaveBeenCalledWith("usr_maya");
    expect(usersApi.disableUser).not.toHaveBeenCalled();
    expect(done).toHaveBeenCalledWith({ kind: "enable", user: disabled });
  });

  it("warns that unlinking locks out someone who has no password", () => {
    const actions = useAccountActions(vi.fn());

    actions.ask("unlink", userFixture({ ssoLinked: true, hasPassword: false }));
    expect(actions.copy.value?.title).toBe(
      "Unlink single sign-on for maya@example.com?",
    );
    expect(actions.copy.value?.description).toContain("can't sign in");

    actions.ask("unlink", userFixture({ ssoLinked: true, hasPassword: true }));
    expect(actions.copy.value?.description).toBe(
      "They sign in with their password until they link single sign-on again.",
    );
  });

  it("unlinks single sign-on once confirmed", async () => {
    const actions = useAccountActions(vi.fn());
    actions.ask("unlink", userFixture({ ssoLinked: true }));
    await actions.confirm();
    expect(usersApi.unlinkUserSso).toHaveBeenCalledWith("usr_maya");
  });

  it("asks for the email before deleting an account", async () => {
    const done = vi.fn();
    const actions = useAccountActions(done);

    actions.ask("delete", maya);
    expect(actions.copy.value).toMatchObject({
      title: "Delete maya@example.com?",
      confirmLabel: "Delete account",
      tone: "danger",
      confirmWord: "maya@example.com",
    });
    expect(actions.copy.value?.description).toContain("can't be undone");

    await actions.confirm();
    expect(usersApi.deleteUser).toHaveBeenCalledWith("usr_maya");
    expect(done).toHaveBeenCalledWith({ kind: "delete", user: maya });
  });

  it("closes the confirmation on cancel without touching the account", () => {
    const actions = useAccountActions(vi.fn());
    actions.ask("delete", maya);
    actions.cancel();
    expect(actions.pending.value).toBeNull();
    expect(usersApi.deleteUser).not.toHaveBeenCalled();
  });

  it("keeps the confirmation open while the request is still running", async () => {
    let finish!: () => void;
    vi.mocked(usersApi.deleteUser).mockReturnValue(
      new Promise<void>((resolve) => (finish = resolve)),
    );
    const actions = useAccountActions(vi.fn());

    actions.ask("delete", maya);
    const running = actions.confirm();
    expect(actions.busy.value).toBe(true);
    actions.cancel();
    expect(actions.pending.value).not.toBeNull();

    finish();
    await running;
    expect(actions.busy.value).toBe(false);
    expect(actions.pending.value).toBeNull();
  });

  it("explains that someone who owns AI agents can't be deleted yet", async () => {
    vi.mocked(usersApi.deleteUser).mockRejectedValue(
      new ApiError(409, { OWNS_AI_AGENTS: "The account owns AI agents." }),
    );
    const done = vi.fn();
    const actions = useAccountActions(done);

    actions.ask("delete", maya);
    await actions.confirm();

    expect(actions.error.value).toBe(
      "They own AI agents. Move those agents to someone else on the AI agents page first.",
    );
    expect(actions.pending.value).toEqual({ kind: "delete", user: maya });
    expect(actions.busy.value).toBe(false);
    expect(done).not.toHaveBeenCalled();
  });

  it("shows the server's reason when an action is refused", async () => {
    vi.mocked(usersApi.disableUser).mockRejectedValue(
      new ApiError(403, { CANNOT_DISABLE_SELF: "You can't disable yourself." }),
    );
    const actions = useAccountActions(vi.fn());

    actions.ask("disable", maya);
    await actions.confirm();

    expect(actions.error.value).toBe("You can't disable yourself.");
  });

  it("falls back to a retry hint when the failure says nothing useful", async () => {
    vi.mocked(usersApi.enableUser).mockRejectedValue("boom");
    const actions = useAccountActions(vi.fn());

    actions.ask("enable", maya);
    await actions.confirm();

    expect(actions.error.value).toBe("That didn't work. Try again.");
  });

  it("clears the last error when another action is asked", async () => {
    vi.mocked(usersApi.disableUser).mockRejectedValue(
      new ApiError(500, { INTERNAL: "Something broke." }),
    );
    const actions = useAccountActions(vi.fn());
    actions.ask("disable", maya);
    await actions.confirm();
    expect(actions.error.value).toBe("Something broke.");

    actions.ask("delete", maya);
    expect(actions.error.value).toBeNull();
  });

  it("does nothing when confirm runs with nothing pending", async () => {
    const done = vi.fn();
    const actions = useAccountActions(done);
    await actions.confirm();
    expect(actions.busy.value).toBe(false);
    expect(done).not.toHaveBeenCalled();
  });

  it("waits for a password confirmation and stays quiet when it's dismissed", async () => {
    vi.mocked(usersApi.disableUser).mockRejectedValue(challenge());
    const done = vi.fn();
    const actions = useAccountActions(done);

    actions.ask("disable", maya);
    const running = actions.confirm();
    await settle();
    expect(actions.busy.value).toBe(true);

    useReauthentication().dismiss();
    await running;

    expect(actions.error.value).toBeNull();
    expect(actions.busy.value).toBe(false);
    expect(actions.pending.value).toEqual({ kind: "disable", user: maya });
    expect(done).not.toHaveBeenCalled();
  });
});
