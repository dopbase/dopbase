import { beforeEach, describe, expect, it, vi } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import { nextTick, ref } from "vue";
import { useUserDialog } from "./UserDialog.controller";
import * as usersApi from "~/services/users.api";
import type { User } from "~/services/users.api";
import type { Role } from "~/services/roles.api";
import { ApiError } from "~/services/http.client";
import { mountController } from "~/tests/mount-controller";
import {
  adminRole,
  agentRole,
  maya,
  memberRole,
  userFixture,
  viewerRole,
} from "~/tests/people";

vi.mock("~/services/users.api");

const PASSWORD = "correct horse battery";

beforeEach(() => {
  setActivePinia(createPinia());
  vi.clearAllMocks();
  vi.mocked(usersApi.createUser).mockImplementation(async (email) =>
    userFixture({ id: "usr_new", email }),
  );
  vi.mocked(usersApi.updateUser).mockImplementation(async (id, body) =>
    userFixture({ id, ...body }),
  );
});

async function openDialog({
  editing = null as User | null,
  roles = [adminRole, memberRole, viewerRole, agentRole] as Role[],
  sso = false,
} = {}) {
  const open = ref(false);
  const editingRef = ref<User | null>(editing);
  const rolesRef = ref<Role[]>(roles);
  const ssoEnabled = ref(sso);
  const { controller } = mountController(() =>
    useUserDialog(open, editingRef, rolesRef, ssoEnabled),
  );
  open.value = true;
  await nextTick();
  return { dialog: controller, open, editing: editingRef };
}

describe("useUserDialog", () => {
  describe("opening", () => {
    it("starts a new account empty, with a password, on the Member role", async () => {
      const { dialog } = await openDialog();
      expect(dialog.email.value).toBe("");
      expect(dialog.password.value).toBe("");
      expect(dialog.signIn.value).toBe("password");
      expect(dialog.roleId.value).toBe("rol_member");
    });

    it("picks the first role you can give when Member is out of reach", async () => {
      const { dialog } = await openDialog({
        roles: [
          { ...adminRole, assignable: false },
          { ...memberRole, assignable: false },
          viewerRole,
        ],
      });
      expect(dialog.roleId.value).toBe("rol_viewer");
    });

    it("fills in the account being edited", async () => {
      const { dialog } = await openDialog({ editing: maya });
      expect(dialog.email.value).toBe("maya@example.com");
      expect(dialog.roleId.value).toBe("rol_member");
    });

    it("clears what was typed and the errors from last time", async () => {
      const { dialog, open } = await openDialog();
      dialog.password.value = "short";
      dialog.signIn.value = "sso";
      await dialog.submit();
      expect(dialog.errors.value).not.toEqual({});

      open.value = false;
      await nextTick();
      open.value = true;
      await nextTick();

      expect(dialog.errors.value).toEqual({});
      expect(dialog.formError.value).toBeNull();
      expect(dialog.password.value).toBe("");
      expect(dialog.signIn.value).toBe("password");
    });
  });

  describe("sign-in choice", () => {
    it("offers single sign-on when adding someone while it's on", async () => {
      const { dialog } = await openDialog({ sso: true });
      expect(dialog.canChooseSso.value).toBe(true);
    });

    it("doesn't offer single sign-on while it's off", async () => {
      const { dialog } = await openDialog({ sso: false });
      expect(dialog.canChooseSso.value).toBe(false);
    });

    it("doesn't offer single sign-on when editing", async () => {
      const { dialog } = await openDialog({ editing: maya, sso: true });
      expect(dialog.canChooseSso.value).toBe(false);
    });
  });

  describe("role choices", () => {
    it("offers only roles people sign in with, leaving AI agents out", async () => {
      const { dialog } = await openDialog();
      expect(dialog.roleChoices.value.map((choice) => choice.id)).toEqual([
        "rol_admin",
        "rol_member",
        "rol_viewer",
      ]);
    });

    it("shows roles beyond your own but won't let you pick them", async () => {
      const { dialog } = await openDialog({
        roles: [
          {
            ...adminRole,
            assignable: false,
            description: "Runs the instance.",
          },
          { ...viewerRole, assignable: false },
          memberRole,
        ],
      });
      expect(dialog.roleChoices.value).toEqual([
        {
          id: "rol_admin",
          name: "Admin",
          description: "Runs the instance. Your role can't give this one.",
          disabled: true,
        },
        {
          id: "rol_viewer",
          name: "Viewer",
          description: "Your role can't give this one.",
          disabled: true,
        },
        {
          id: "rol_member",
          name: "Member",
          description: "",
          disabled: false,
        },
      ]);
    });

    it("keeps someone's current role selectable even if you couldn't give it", async () => {
      const lockedAdmin = { ...adminRole, assignable: false };
      const { dialog } = await openDialog({
        editing: userFixture({ roleId: "rol_admin" }),
        roles: [lockedAdmin, memberRole],
      });
      expect(
        dialog.roleChoices.value.find((choice) => choice.id === "rol_admin")
          ?.disabled,
      ).toBe(false);
      expect(dialog.roleId.value).toBe("rol_admin");
    });
  });

  describe("validation", () => {
    it("asks for an email", async () => {
      const { dialog } = await openDialog();
      dialog.password.value = PASSWORD;
      expect(await dialog.submit()).toBeNull();
      expect(dialog.errors.value).toEqual({ email: "Enter their email." });
      expect(usersApi.createUser).not.toHaveBeenCalled();
    });

    it("rejects something that isn't an email", async () => {
      const { dialog } = await openDialog();
      dialog.email.value = "maya";
      dialog.password.value = PASSWORD;
      await dialog.submit();
      expect(dialog.errors.value.email).toBe(
        "That doesn't look like an email address.",
      );
    });

    it("needs a temporary password for someone signing in with one", async () => {
      const { dialog } = await openDialog();
      dialog.email.value = "sam@example.com";
      await dialog.submit();
      expect(dialog.errors.value).toEqual({
        password: "Set a temporary password.",
      });
    });

    it("holds passwords to the length limits", async () => {
      const { dialog } = await openDialog();
      dialog.email.value = "sam@example.com";

      dialog.password.value = "short";
      await dialog.submit();
      expect(dialog.errors.value.password).toBe("Use at least 12 characters.");

      dialog.password.value = "x".repeat(129);
      await dialog.submit();
      expect(dialog.errors.value.password).toBe("Use at most 128 characters.");
      expect(usersApi.createUser).not.toHaveBeenCalled();
    });

    it("needs no password for someone who signs in with single sign-on", async () => {
      const { dialog } = await openDialog({ sso: true });
      dialog.email.value = "sam@example.com";
      dialog.signIn.value = "sso";
      await dialog.submit();
      expect(dialog.errors.value).toEqual({});
    });

    it("asks for a role when there's none to pick", async () => {
      const { dialog } = await openDialog({ roles: [agentRole] });
      dialog.email.value = "sam@example.com";
      dialog.password.value = PASSWORD;
      await dialog.submit();
      expect(dialog.errors.value).toEqual({ role: "Pick a role." });
    });

    it("leaves the password alone on edit unless a new one is typed", async () => {
      const { dialog } = await openDialog({ editing: maya });
      expect(await dialog.submit()).not.toBeNull();

      dialog.password.value = "short";
      expect(await dialog.submit()).toBeNull();
      expect(dialog.errors.value.password).toBe("Use at least 12 characters.");
    });
  });

  describe("notes", () => {
    it("tells you a new account reaches nothing until it gets access", async () => {
      const { dialog } = await openDialog();
      dialog.email.value = "  sam@example.com ";
      expect(dialog.note.value).toBe(
        "sam@example.com reaches no project until someone gives them access. You can do that from their page right after this.",
      );
    });

    it("says the same before an email is typed", async () => {
      const { dialog } = await openDialog();
      expect(dialog.note.value).toBe(
        "They reach no project until someone gives them access. You can do that from their page right after this.",
      );
    });

    it("warns that changing the role signs them out", async () => {
      const { dialog } = await openDialog({ editing: maya });
      expect(dialog.note.value).toBe("");
      dialog.roleId.value = "rol_viewer";
      expect(dialog.note.value).toContain("signs them out everywhere");
    });
  });

  describe("saving", () => {
    it("creates an account with a temporary password and the chosen role", async () => {
      const { dialog } = await openDialog();
      dialog.email.value = " sam@example.com ";
      dialog.password.value = PASSWORD;
      dialog.roleId.value = "rol_viewer";

      const saved = await dialog.submit();

      expect(usersApi.createUser).toHaveBeenCalledWith(
        "sam@example.com",
        PASSWORD,
        { roleId: "rol_viewer" },
      );
      expect(saved?.id).toBe("usr_new");
      expect(dialog.saving.value).toBe(false);
    });

    it("creates a single sign-on account without a password", async () => {
      const { dialog } = await openDialog({ sso: true });
      dialog.email.value = "sam@example.com";
      dialog.signIn.value = "sso";

      await dialog.submit();

      expect(usersApi.createUser).toHaveBeenCalledWith("sam@example.com", "", {
        roleId: "rol_member",
      });
    });

    it("sends only what changed on edit", async () => {
      const { dialog } = await openDialog({ editing: maya });
      dialog.roleId.value = "rol_viewer";
      await dialog.submit();
      expect(usersApi.updateUser).toHaveBeenLastCalledWith("usr_maya", {
        roleId: "rol_viewer",
      });

      dialog.roleId.value = "rol_member";
      dialog.email.value = "maya@corp.example";
      dialog.password.value = PASSWORD;
      await dialog.submit();
      expect(usersApi.updateUser).toHaveBeenLastCalledWith("usr_maya", {
        email: "maya@corp.example",
        password: PASSWORD,
      });
      expect(usersApi.createUser).not.toHaveBeenCalled();
    });

    it("shows a taken email next to the email field", async () => {
      vi.mocked(usersApi.createUser).mockRejectedValue(
        new ApiError(409, { EMAIL_TAKEN: "Someone already uses that email." }),
      );
      const { dialog } = await openDialog();
      dialog.email.value = "maya@example.com";
      dialog.password.value = PASSWORD;

      expect(await dialog.submit()).toBeNull();
      expect(dialog.errors.value.email).toBe(
        "Someone already uses that email.",
      );
      expect(dialog.formError.value).toBeNull();
    });

    it("shows the server's password rule next to the password field", async () => {
      vi.mocked(usersApi.updateUser).mockRejectedValue(
        new ApiError(400, { PASSWORD_TOO_SHORT: "Too short for the server." }),
      );
      const { dialog } = await openDialog({ editing: maya });
      dialog.password.value = PASSWORD;

      await dialog.submit();
      expect(dialog.errors.value.password).toBe("Too short for the server.");
    });

    it("shows any other refusal above the form", async () => {
      vi.mocked(usersApi.createUser).mockRejectedValue(
        new ApiError(403, { ROLE_NOT_ASSIGNABLE: "You can't give that role." }),
      );
      const { dialog } = await openDialog();
      dialog.email.value = "sam@example.com";
      dialog.password.value = PASSWORD;

      await dialog.submit();
      expect(dialog.formError.value).toBe("You can't give that role.");
    });

    it("says the account wasn't saved when the failure isn't from the server", async () => {
      vi.mocked(usersApi.createUser).mockRejectedValue(new TypeError("fetch"));
      const { dialog } = await openDialog();
      dialog.email.value = "sam@example.com";
      dialog.password.value = PASSWORD;

      expect(await dialog.submit()).toBeNull();
      expect(dialog.formError.value).toBe("The account could not be saved.");
      expect(dialog.saving.value).toBe(false);
    });
  });
});
