import { beforeEach, describe, expect, it, vi } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import { nextTick, ref } from "vue";
import { useTransferOwnerDialog } from "./TransferOwnerDialog.controller";
import * as usersApi from "~/services/users.api";
import type { User } from "~/services/users.api";
import { ApiError } from "~/services/http.client";
import { mountController } from "~/tests/mount-controller";
import {
  lead,
  maya,
  reader,
  reviewBot,
  signInAs,
  userFixture,
} from "~/tests/people";

vi.mock("~/services/users.api");

const departed = userFixture({
  id: "usr_gone",
  email: "gone@example.com",
  status: "disabled",
});
const otherBot = userFixture({
  ...reviewBot,
  id: "usr_other",
  email: "other-bot",
});

async function openDialog(
  agent: User | null = reviewBot,
  people: User[] = [maya, departed, otherBot, lead, reader],
) {
  const open = ref(false);
  const { controller } = mountController(() =>
    useTransferOwnerDialog(open, ref(agent), ref(people)),
  );
  open.value = true;
  await nextTick();
  return { open, dialog: controller };
}

beforeEach(() => {
  setActivePinia(createPinia());
  vi.clearAllMocks();
  signInAs(["users.manage"]);
  vi.mocked(usersApi.updateUser).mockResolvedValue(reviewBot);
});

describe("useTransferOwnerDialog", () => {
  it("offers active people other than the current owner, and no agents", async () => {
    const { dialog } = await openDialog();

    expect(dialog.options.value).toEqual([
      { value: "usr_lead", label: "lead@example.com" },
      { value: "usr_reader", label: "reader@example.com" },
    ]);
  });

  it("preselects the first person it can move the agent to", async () => {
    const { dialog } = await openDialog();

    expect(dialog.ownerId.value).toBe("usr_lead");
  });

  it("asks for a new owner when nobody else can take the agent", async () => {
    const { dialog } = await openDialog(reviewBot, [maya, departed]);

    expect(await dialog.submit()).toBe(false);
    expect(dialog.error.value).toBe("Choose the new owner.");
    expect(usersApi.updateUser).not.toHaveBeenCalled();
  });

  it("moves the agent to the chosen owner", async () => {
    const { dialog } = await openDialog();
    dialog.ownerId.value = "usr_reader";

    expect(await dialog.submit()).toBe(true);
    expect(usersApi.updateUser).toHaveBeenCalledWith("usr_bot", {
      ownerId: "usr_reader",
    });
    expect(dialog.saving.value).toBe(false);
  });

  it("shows the server's reason when the transfer fails", async () => {
    vi.mocked(usersApi.updateUser).mockRejectedValue(
      new ApiError(403, {
        OWNER_TOO_LOW: "They reach less than the agent does.",
      }),
    );
    const { dialog } = await openDialog();

    expect(await dialog.submit()).toBe(false);
    expect(dialog.error.value).toBe("They reach less than the agent does.");
    expect(dialog.saving.value).toBe(false);
  });

  it("clears an earlier error when it opens again", async () => {
    const { open, dialog } = await openDialog(reviewBot, [maya]);
    await dialog.submit();

    open.value = false;
    await nextTick();
    open.value = true;
    await nextTick();

    expect(dialog.error.value).toBeNull();
  });
});
