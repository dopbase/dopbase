import { beforeEach, describe, expect, it, vi } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import { nextTick, ref } from "vue";
import {
  useAddMembersDialog,
  type MembersTarget,
} from "./AddMembersDialog.controller";
import * as accessApi from "~/services/access.api";
import { ApiError } from "~/services/http.client";
import {
  groupDetailFixture,
  lead,
  maya,
  reader,
  reviewBot,
  root,
} from "~/tests/people";

vi.mock("~/services/access.api", async (original) => ({
  ...(await original<typeof accessApi>()),
  addGroupMember: vi.fn(),
}));

const backend: MembersTarget = {
  id: "grp_backend",
  name: "Backend",
  memberIds: ["usr_maya"],
};

beforeEach(() => {
  setActivePinia(createPinia());
  vi.clearAllMocks();
  vi.mocked(accessApi.addGroupMember).mockResolvedValue(groupDetailFixture());
});

function open(target: MembersTarget | null = backend) {
  return useAddMembersDialog(
    ref(true),
    ref(target),
    ref([root, lead, maya, reader, reviewBot]),
  );
}

describe("useAddMembersDialog", () => {
  it("offers everyone but root and the people already in the group", () => {
    const dialog = open();
    expect(dialog.candidates.value.map((person) => person.id)).toEqual([
      "usr_lead",
      "usr_reader",
      "usr_bot",
    ]);
  });

  it("filters the list by email", () => {
    const dialog = open();
    dialog.filter.value = "READ";
    expect(dialog.visible.value.map((person) => person.id)).toEqual([
      "usr_reader",
    ]);
  });

  it("counts the picks on the button", () => {
    const dialog = open();
    expect(dialog.submitLabel.value).toBe("Add members");
    dialog.toggle("usr_lead", true);
    expect(dialog.submitLabel.value).toBe("Add 1 member");
    dialog.toggle("usr_reader", true);
    expect(dialog.submitLabel.value).toBe("Add 2 members");
    dialog.toggle("usr_lead", false);
    expect(dialog.submitLabel.value).toBe("Add 1 member");
  });

  it("adds each pick to the group", async () => {
    const dialog = open();
    dialog.toggle("usr_lead", true);
    dialog.toggle("usr_bot", true);
    expect(await dialog.submit()).toBe(true);
    expect(vi.mocked(accessApi.addGroupMember).mock.calls).toEqual([
      ["grp_backend", "usr_lead"],
      ["grp_backend", "usr_bot"],
    ]);
  });

  it("asks for at least one pick", async () => {
    const dialog = open();
    expect(await dialog.submit()).toBe(false);
    expect(dialog.error.value).toBe("Pick at least one person.");
    expect(accessApi.addGroupMember).not.toHaveBeenCalled();
  });

  it("stops at the first failure and says so", async () => {
    vi.mocked(accessApi.addGroupMember)
      .mockResolvedValueOnce(groupDetailFixture())
      .mockRejectedValueOnce(
        new ApiError(422, { GROUP_MEMBER_INVALID: "Root can't join groups." }),
      );
    const dialog = open();
    dialog.toggle("usr_lead", true);
    dialog.toggle("usr_reader", true);
    dialog.toggle("usr_bot", true);
    expect(await dialog.submit()).toBe(false);
    expect(accessApi.addGroupMember).toHaveBeenCalledTimes(2);
    expect(dialog.error.value).toBe("Root can't join groups.");
    expect(dialog.saving.value).toBe(false);
  });

  it("does nothing without a group", async () => {
    const dialog = open(null);
    expect(await dialog.submit()).toBe(false);
    expect(accessApi.addGroupMember).not.toHaveBeenCalled();
  });

  it("forgets picks and the filter when it opens again", async () => {
    const isOpen = ref(true);
    const dialog = useAddMembersDialog(isOpen, ref(backend), ref([lead]));
    dialog.toggle("usr_lead", true);
    dialog.filter.value = "lead";
    isOpen.value = false;
    await nextTick();
    isOpen.value = true;
    await nextTick();
    expect(dialog.selected.value.size).toBe(0);
    expect(dialog.filter.value).toBe("");
  });
});
