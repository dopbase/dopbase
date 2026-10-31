import { beforeEach, describe, expect, it, vi } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import { nextTick, ref } from "vue";
import { useAddAgentDialog } from "./AddAgentDialog.controller";
import * as usersApi from "~/services/users.api";
import type { Role } from "~/services/roles.api";
import { ApiError } from "~/services/http.client";
import { mountController } from "~/tests/mount-controller";
import { agentRole, reviewBot, signInAs } from "~/tests/people";

vi.mock("~/services/users.api");

async function openDialog(role: Role | null = agentRole) {
  const open = ref(false);
  const { controller } = mountController(() =>
    useAddAgentDialog(open, ref(role)),
  );
  open.value = true;
  await nextTick();
  return { open, dialog: controller };
}

beforeEach(() => {
  setActivePinia(createPinia());
  vi.clearAllMocks();
  signInAs(["users.manage"]);
  vi.mocked(usersApi.createUser).mockResolvedValue(reviewBot);
});

describe("useAddAgentDialog", () => {
  it("asks for a name before adding anything", async () => {
    const { dialog } = await openDialog();
    dialog.name.value = "   ";

    expect(await dialog.submit()).toBeNull();
    expect(dialog.nameError.value).toBe(
      "Name the agent after what it does, like review-bot.",
    );
    expect(usersApi.createUser).not.toHaveBeenCalled();
  });

  it("says so when there is no AI agent role you can give", async () => {
    const { dialog } = await openDialog(null);
    dialog.name.value = "review-bot";

    expect(await dialog.submit()).toBeNull();
    expect(dialog.error.value).toBe("There's no AI agent role you can give.");
    expect(usersApi.createUser).not.toHaveBeenCalled();
  });

  it("adds the agent with no password and the agent role", async () => {
    const { dialog } = await openDialog();
    dialog.name.value = " review-bot ";

    expect(await dialog.submit()).toEqual(reviewBot);
    expect(usersApi.createUser).toHaveBeenCalledWith("review-bot", "", {
      roleId: "rol_ai_agent",
    });
    expect(dialog.saving.value).toBe(false);
  });

  it("puts a taken name under the name field", async () => {
    vi.mocked(usersApi.createUser).mockRejectedValue(
      new ApiError(409, { EMAIL_TAKEN: "That name is taken." }),
    );
    const { dialog } = await openDialog();
    dialog.name.value = "review-bot";

    expect(await dialog.submit()).toBeNull();
    expect(dialog.nameError.value).toBe("That name is taken.");
    expect(dialog.error.value).toBeNull();
  });

  it("puts an invalid name under the name field", async () => {
    vi.mocked(usersApi.createUser).mockRejectedValue(
      new ApiError(422, { EMAIL_INVALID: "Use letters, numbers and hyphens." }),
    );
    const { dialog } = await openDialog();
    dialog.name.value = "review bot";

    await dialog.submit();

    expect(dialog.nameError.value).toBe("Use letters, numbers and hyphens.");
  });

  it("shows any other failure for the whole form", async () => {
    vi.mocked(usersApi.createUser).mockRejectedValue(
      new ApiError(403, { ROLE_NOT_ASSIGNABLE: "You can't give that role." }),
    );
    const { dialog } = await openDialog();
    dialog.name.value = "review-bot";

    expect(await dialog.submit()).toBeNull();
    expect(dialog.error.value).toBe("You can't give that role.");
    expect(dialog.nameError.value).toBeNull();
    expect(dialog.saving.value).toBe(false);
  });

  it("clears the name and errors each time it opens", async () => {
    const { open, dialog } = await openDialog();
    dialog.name.value = "";
    await dialog.submit();
    dialog.name.value = "half-typed";

    open.value = false;
    await nextTick();
    open.value = true;
    await nextTick();

    expect(dialog.name.value).toBe("");
    expect(dialog.nameError.value).toBeNull();
    expect(dialog.error.value).toBeNull();
  });
});
