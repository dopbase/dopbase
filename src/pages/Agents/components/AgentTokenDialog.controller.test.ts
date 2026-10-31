import { beforeEach, describe, expect, it, vi } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import { nextTick, ref } from "vue";
import { useAgentTokenDialog } from "./AgentTokenDialog.controller";
import * as tokensApi from "~/services/personal-tokens.api";
import type { Grant } from "~/services/access.api";
import type { Role } from "~/services/roles.api";
import { ApiError } from "~/services/http.client";
import { mountController } from "~/tests/mount-controller";
import {
  agentRole,
  grantFixture,
  reviewBot,
  roleFixture,
  signInAs,
  tokenFixture,
} from "~/tests/people";

vi.mock("~/services/personal-tokens.api");

const projectGrant = grantFixture({
  id: "grt_pay",
  principalId: "usr_bot",
  level: "read",
});
const envGrant = grantFixture({
  id: "grt_prod",
  principalId: "usr_bot",
  scopeType: "environment",
  scopeId: "env_prod",
  scopeLabel: "payments/prod",
  level: "read",
});

/** Mounts the dialog closed, then opens it so the form resets. */
async function openDialog({
  role = agentRole as Role | null,
  grants = [projectGrant, envGrant] as Grant[],
  current = null as tokensApi.PersonalToken | null,
} = {}) {
  const open = ref(false);
  const { controller } = mountController(() =>
    useAgentTokenDialog(
      open,
      ref(reviewBot),
      ref(role),
      ref(grants),
      ref(current),
    ),
  );
  open.value = true;
  await nextTick();
  return { open, dialog: controller };
}

beforeEach(() => {
  setActivePinia(createPinia());
  vi.clearAllMocks();
  signInAs(["users.manage"]);
  vi.mocked(tokensApi.createPersonalToken).mockResolvedValue({
    token: tokenFixture(),
    plaintextToken: "dop_pat_x",
  });
});

describe("useAgentTokenDialog", () => {
  it("asks for a name before creating anything", async () => {
    const { dialog } = await openDialog();

    expect(await dialog.submit()).toBe(false);
    expect(dialog.nameError.value).toBe(
      "Name the token after where it's used, like ci-deploy.",
    );
    expect(tokensApi.createPersonalToken).not.toHaveBeenCalled();
  });

  it("refuses a name that isn't a slug", async () => {
    const { dialog } = await openDialog();
    dialog.name.value = "CI Deploy";

    expect(await dialog.submit()).toBe(false);
    expect(dialog.nameError.value).toBe(
      "Use lowercase letters, numbers and single hyphens, starting with a letter.",
    );
  });

  it("disables Read when the role's tokens may only see names", async () => {
    const { dialog } = await openDialog({
      role: roleFixture({ ...agentRole, tokenMaxLevel: "metadata" }),
    });

    expect(
      dialog.levels.value.map(({ level, disabled }) => ({ level, disabled })),
    ).toEqual([
      { level: "metadata", disabled: false },
      { level: "read", disabled: true },
    ]);
  });

  it("offers everything the agent reaches, then each project or environment it holds", async () => {
    const hidden = grantFixture({
      id: "grt_hidden",
      scopeType: "environment",
      scopeId: "env_dev",
      scopeLabel: "payments/dev",
      level: "none",
    });
    const audit = grantFixture({
      id: "grt_audit",
      scopeType: "audit",
      scopeId: "",
    });
    const { dialog } = await openDialog({
      grants: [projectGrant, hidden, envGrant, audit],
    });

    expect(dialog.scopeOptions.value).toEqual([
      {
        value: "all",
        label: "Every project review-bot reaches",
        scopeType: "all",
      },
      {
        value: "project:prj_pay",
        label: "payments, whole project",
        scopeType: "project",
        scope: "prj_pay",
      },
      {
        value: "environment:env_prod",
        label: "payments/prod",
        scopeType: "environment",
        scope: "env_prod",
      },
    ]);
  });

  it("only offers lifetimes the agent role allows", async () => {
    const { dialog } = await openDialog();

    expect(dialog.maxDays.value).toBe(90);
    expect(dialog.expiryOptions.value.map((option) => option.value)).toEqual([
      "7d",
      "30d",
      "90d",
      "custom",
    ]);
    expect(dialog.expiry.value).toBe("30d");
  });

  it("starts on 7 days when the role keeps tokens shorter than a month", async () => {
    const { dialog } = await openDialog({
      role: roleFixture({ ...agentRole, tokenMaxTtlDays: 14 }),
    });

    expect(dialog.expiryOptions.value.map((option) => option.value)).toEqual([
      "7d",
      "custom",
    ]);
    expect(dialog.expiry.value).toBe("7d");
  });

  it("assumes 90 days at most when the role is unknown", async () => {
    const { dialog } = await openDialog({ role: null });

    expect(dialog.maxDays.value).toBe(90);
    expect(dialog.levels.value.every((choice) => !choice.disabled)).toBe(true);
  });

  it("clears the form each time it opens", async () => {
    const { open, dialog } = await openDialog();
    dialog.name.value = "Bad Name";
    dialog.level.value = "read";
    dialog.where.value = "project:prj_pay";
    dialog.expiry.value = "custom";
    dialog.customDays.value = "12";
    await dialog.submit();

    open.value = false;
    await nextTick();
    open.value = true;
    await nextTick();

    expect(dialog.name.value).toBe("");
    expect(dialog.level.value).toBe("metadata");
    expect(dialog.where.value).toBe("all");
    expect(dialog.expiry.value).toBe("30d");
    expect(dialog.customDays.value).toBe("");
    expect(dialog.nameError.value).toBeNull();
  });

  it("refuses custom days that aren't a whole number", async () => {
    const { dialog } = await openDialog();
    dialog.name.value = "ci-deploy";
    dialog.expiry.value = "custom";
    dialog.customDays.value = "two weeks";

    expect(await dialog.submit()).toBe(false);
    expect(dialog.expiryError.value).toBe(
      "Enter a whole number of hours or days.",
    );
  });

  it("refuses custom days beyond what the role allows", async () => {
    const { dialog } = await openDialog();
    dialog.name.value = "ci-deploy";
    dialog.expiry.value = "custom";
    dialog.customDays.value = "120";

    expect(await dialog.submit()).toBe(false);
    expect(dialog.expiryError.value).toBe("Their role allows at most 90 days.");
    expect(tokensApi.createPersonalToken).not.toHaveBeenCalled();
  });

  it("names the live token the new one replaces", async () => {
    const { dialog } = await openDialog({
      current: tokenFixture({ name: "old-ci" }),
    });

    expect(dialog.replaces.value).toBe("old-ci");
  });

  it("replaces nothing when the agent has no live token", async () => {
    const { dialog } = await openDialog();

    expect(dialog.replaces.value).toBeNull();
  });

  it("creates a token narrowed to one environment that replaces the live one", async () => {
    const { dialog } = await openDialog({ current: tokenFixture() });
    dialog.name.value = " ci-deploy ";
    dialog.level.value = "read";
    dialog.where.value = "environment:env_prod";
    dialog.expiry.value = "90d";

    expect(await dialog.submit()).toBe(true);
    expect(tokensApi.createPersonalToken).toHaveBeenCalledWith("usr_bot", {
      name: "ci-deploy",
      level: "read",
      scopeType: "environment",
      scope: "env_prod",
      expiresIn: "90d",
      replaceExisting: true,
    });
    expect(dialog.created.value?.plaintextToken).toBe("dop_pat_x");
    expect(dialog.saving.value).toBe(false);
  });

  it("sends no scope for a token that reaches everything, with custom days", async () => {
    const { dialog } = await openDialog();
    dialog.name.value = "nightly";
    dialog.expiry.value = "custom";
    dialog.customDays.value = "45";

    await dialog.submit();

    expect(tokensApi.createPersonalToken).toHaveBeenCalledWith("usr_bot", {
      name: "nightly",
      level: "metadata",
      expiresIn: "45d",
      replaceExisting: true,
    });
  });

  it("flags the name when the agent already has a token called that", async () => {
    vi.mocked(tokensApi.createPersonalToken).mockRejectedValue(
      new ApiError(409, { TOKEN_NAME_TAKEN: "Name taken." }),
    );
    const { dialog } = await openDialog();
    dialog.name.value = "ci-deploy";

    expect(await dialog.submit()).toBe(false);
    expect(dialog.nameError.value).toBe(
      "This agent already has a token with that name.",
    );
    expect(dialog.error.value).toBeNull();
  });

  it("shows the server's reason for any other failure", async () => {
    vi.mocked(tokensApi.createPersonalToken).mockRejectedValue(
      new ApiError(403, { LEVEL_TOO_HIGH: "The role can't hold Read tokens." }),
    );
    const { dialog } = await openDialog();
    dialog.name.value = "ci-deploy";

    expect(await dialog.submit()).toBe(false);
    expect(dialog.error.value).toBe("The role can't hold Read tokens.");
    expect(dialog.saving.value).toBe(false);
  });
});
