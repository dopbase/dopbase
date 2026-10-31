import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import { nextTick, ref } from "vue";
import {
  usePersonalTokenDialog,
  type TokenLimits,
} from "./PersonalTokenDialog.controller";
import * as tokensApi from "~/services/personal-tokens.api";
import type { Environment } from "~/services/environments.api";
import { ApiError } from "~/services/http.client";
import { mountController } from "~/tests/mount-controller";
import { signInAs, tokenFixture } from "~/tests/people";

vi.mock("~/services/personal-tokens.api");

const payments = {
  id: "prj_pay",
  name: "payments",
  createdAt: "",
  updatedAt: "",
};
const prod: Environment = {
  id: "env_prod",
  projectId: "prj_pay",
  projectName: "payments",
  name: "prod",
  createdAt: "",
  updatedAt: "",
};

/** Mounts the dialog closed, then opens it so the form resets. */
async function openDialog({
  auditOnly = false,
  limits = { level: "write", days: 365 } as TokenLimits,
} = {}) {
  const open = ref(false);
  const { controller } = mountController(() =>
    usePersonalTokenDialog(
      open,
      ref(auditOnly),
      ref([payments]),
      ref([prod]),
      ref(limits),
    ),
  );
  open.value = true;
  await nextTick();
  return { open, dialog: controller };
}

beforeEach(() => {
  setActivePinia(createPinia());
  vi.clearAllMocks();
  vi.useFakeTimers({ toFake: ["Date"] });
  vi.setSystemTime(new Date("2026-10-27T12:00:00Z"));
  signInAs(["tokens.personal"], { adminId: "usr_maya", role: "member" });
  vi.mocked(tokensApi.createPersonalToken).mockResolvedValue({
    token: tokenFixture(),
    plaintextToken: "dop_pat_x",
  });
});

afterEach(() => {
  vi.useRealTimers();
});

describe("usePersonalTokenDialog", () => {
  it("starts from the AI agent preset: Names only for 30 days", async () => {
    const { dialog } = await openDialog();

    expect(dialog.preset.value).toBe("agent");
    expect(dialog.level.value).toBe("metadata");
    expect(dialog.expiry.value).toBe("30d");
    expect(dialog.where.value).toBe("all");
  });

  it("sets Read for 90 days from the Script preset", async () => {
    const { dialog } = await openDialog();

    dialog.choosePreset("script");

    expect(dialog.level.value).toBe("read");
    expect(dialog.expiry.value).toBe("90d");
  });

  it("keeps a preset within what the role allows", async () => {
    const { dialog } = await openDialog({
      limits: { level: "metadata", days: 30 },
    });

    dialog.choosePreset("script");

    expect(dialog.level.value).toBe("metadata");
    expect(dialog.expiry.value).toBe("30d");
    expect(dialog.expiryOptions.value.map((option) => option.value)).toEqual([
      "7d",
      "30d",
      "custom",
    ]);
  });

  it("turns into Custom once you change what the preset set", async () => {
    const { dialog } = await openDialog();

    dialog.level.value = "write";
    await nextTick();

    expect(dialog.preset.value).toBe("custom");
  });

  it("disables levels above what the role allows", async () => {
    const { dialog } = await openDialog({
      limits: { level: "read", days: 90 },
    });

    expect(
      dialog.levels.value.map(({ level, disabled }) => ({ level, disabled })),
    ).toEqual([
      { level: "metadata", disabled: false },
      { level: "read", disabled: false },
      { level: "write", disabled: true },
    ]);
  });

  it("offers every project you reach, then each project and its environments", async () => {
    const { dialog } = await openDialog();

    expect(dialog.scopeOptions.value.map((option) => option.label)).toEqual([
      "Every project you reach",
      "payments",
      "payments/prod",
    ]);
  });

  it("sums up what the token can do, where, and until when", async () => {
    const { dialog } = await openDialog();
    dialog.name.value = "claude-laptop";
    dialog.where.value = "project:prj_pay";

    expect(dialog.summary.value).toBe(
      "claude-laptop can list names in payments until Nov 26, 2026, and never more than you can.",
    );

    dialog.level.value = "write";
    dialog.where.value = "all";
    dialog.expiry.value = "custom";
    dialog.customDays.value = "";
    expect(dialog.summary.value).toBe(
      "claude-laptop can read and change secrets in every project you reach, and never more than you can.",
    );
  });

  it("sums up an audit token", async () => {
    const { dialog } = await openDialog({ auditOnly: true });

    expect(dialog.summary.value).toBe(
      "This token can read the audit log you see until Nov 26, 2026. It never reaches projects or secrets.",
    );
  });

  it("refuses a name that isn't a slug", async () => {
    const { dialog } = await openDialog();

    expect(await dialog.submit()).toBe(false);
    expect(dialog.nameError.value).toBe(
      "Name it after where it's used, like claude-laptop.",
    );
    dialog.name.value = "Claude Laptop";
    expect(await dialog.submit()).toBe(false);
    expect(dialog.nameError.value).toBe(
      "Use lowercase letters, numbers and single hyphens, starting with a letter.",
    );
    expect(tokensApi.createPersonalToken).not.toHaveBeenCalled();
  });

  it("refuses custom days beyond the role's longest lifetime", async () => {
    const { dialog } = await openDialog({
      limits: { level: "read", days: 90 },
    });
    dialog.name.value = "nightly";
    dialog.expiry.value = "custom";
    dialog.customDays.value = "120";

    expect(await dialog.submit()).toBe(false);
    expect(dialog.expiryError.value).toBe("Your role allows at most 90 days.");
  });

  it("creates a token narrowed to one environment", async () => {
    const { dialog } = await openDialog();
    dialog.name.value = " claude-laptop ";
    dialog.level.value = "read";
    dialog.where.value = "environment:env_prod";

    expect(await dialog.submit()).toBe(true);
    expect(tokensApi.createPersonalToken).toHaveBeenCalledWith("me", {
      name: "claude-laptop",
      level: "read",
      expiresIn: "30d",
      scopeType: "environment",
      scope: "env_prod",
    });
    expect(dialog.created.value?.plaintextToken).toBe("dop_pat_x");
  });

  it("creates a token for every project you reach with custom days", async () => {
    const { dialog } = await openDialog();
    dialog.name.value = "deploy-script";
    dialog.expiry.value = "custom";
    dialog.customDays.value = "45";

    await dialog.submit();

    expect(tokensApi.createPersonalToken).toHaveBeenCalledWith("me", {
      name: "deploy-script",
      level: "metadata",
      expiresIn: "45d",
      scopeType: "all",
    });
  });

  it("creates an audit token with no level or place", async () => {
    const { dialog } = await openDialog({ auditOnly: true });
    dialog.name.value = "splunk-forwarder";

    await dialog.submit();

    expect(tokensApi.createPersonalToken).toHaveBeenCalledWith("me", {
      name: "splunk-forwarder",
      scopeType: "audit",
      expiresIn: "30d",
    });
  });

  it("flags the name when you already have a token called that", async () => {
    vi.mocked(tokensApi.createPersonalToken).mockRejectedValue(
      new ApiError(409, { TOKEN_NAME_TAKEN: "Name taken." }),
    );
    const { dialog } = await openDialog();
    dialog.name.value = "claude-laptop";

    expect(await dialog.submit()).toBe(false);
    expect(dialog.nameError.value).toBe(
      "You already have a token with that name.",
    );
    expect(dialog.saving.value).toBe(false);
  });

  it("clears the form each time it opens", async () => {
    const { open, dialog } = await openDialog();
    dialog.name.value = "Bad Name";
    dialog.choosePreset("script");
    dialog.where.value = "project:prj_pay";
    await dialog.submit();

    open.value = false;
    await nextTick();
    open.value = true;
    await nextTick();

    expect(dialog.name.value).toBe("");
    expect(dialog.preset.value).toBe("agent");
    expect(dialog.level.value).toBe("metadata");
    expect(dialog.where.value).toBe("all");
    expect(dialog.nameError.value).toBeNull();
  });
});
