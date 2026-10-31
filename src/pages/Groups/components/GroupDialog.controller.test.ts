import { beforeEach, describe, expect, it, vi } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import { nextTick, ref } from "vue";
import { useGroupDialog } from "./GroupDialog.controller";
import * as accessApi from "~/services/access.api";
import { ApiError } from "~/services/http.client";
import { groupDetailFixture, groupSummaryFixture } from "~/tests/people";

vi.mock("~/services/access.api", async (original) => ({
  ...(await original<typeof accessApi>()),
  createGroup: vi.fn(),
  updateGroup: vi.fn(),
}));

beforeEach(() => {
  setActivePinia(createPinia());
  vi.clearAllMocks();
});

describe("useGroupDialog", () => {
  it("creates a group with a trimmed name and description", async () => {
    const created = groupDetailFixture();
    vi.mocked(accessApi.createGroup).mockResolvedValueOnce(created);
    const dialog = useGroupDialog(ref(true), ref(null));
    expect(dialog.title.value).toBe("New group");
    dialog.name.value = "  Backend ";
    dialog.description.value = " Payments service team ";
    expect(await dialog.submit()).toBe(true);
    expect(accessApi.createGroup).toHaveBeenCalledWith(
      "Backend",
      "Payments service team",
    );
    expect(dialog.saved.value).toStrictEqual(created);
  });

  it("asks for a name before calling the server", async () => {
    const dialog = useGroupDialog(ref(true), ref(null));
    dialog.name.value = "   ";
    expect(await dialog.submit()).toBe(false);
    expect(dialog.nameError.value).toBe("Enter a group name.");
    expect(accessApi.createGroup).not.toHaveBeenCalled();
  });

  it("starts a rename from the group's current name", async () => {
    const group = groupSummaryFixture();
    vi.mocked(accessApi.updateGroup).mockResolvedValueOnce(
      groupDetailFixture(),
    );
    const dialog = useGroupDialog(ref(true), ref(group));
    expect(dialog.title.value).toBe("Rename Backend");
    expect(dialog.name.value).toBe("Backend");
    expect(dialog.description.value).toBe("Payments service team");
    dialog.name.value = "Platform";
    expect(await dialog.submit()).toBe(true);
    expect(accessApi.updateGroup).toHaveBeenCalledWith("grp_backend", {
      name: "Platform",
      description: "Payments service team",
    });
  });

  it("shows the server's reason, such as a name already taken", async () => {
    vi.mocked(accessApi.createGroup).mockRejectedValueOnce(
      new ApiError(409, { GROUP_NAME_TAKEN: "A group with that name exists." }),
    );
    const dialog = useGroupDialog(ref(true), ref(null));
    dialog.name.value = "Backend";
    expect(await dialog.submit()).toBe(false);
    expect(dialog.error.value).toBe("A group with that name exists.");
    expect(dialog.saving.value).toBe(false);
  });

  it("clears the last attempt when it opens again", async () => {
    const open = ref(true);
    const dialog = useGroupDialog(open, ref(null));
    dialog.name.value = "";
    await dialog.submit();
    expect(dialog.nameError.value).not.toBeNull();
    open.value = false;
    await nextTick();
    open.value = true;
    await nextTick();
    expect(dialog.nameError.value).toBeNull();
    expect(dialog.name.value).toBe("");
  });
});
