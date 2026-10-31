import { beforeEach, describe, expect, it, vi } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import { RESET_PHRASE, useInstanceController } from "./Instance.controller";
import * as instanceApi from "~/services/instance.api";
import * as backupsApi from "~/services/backups.api";
import { saveBlob } from "~/utils/download";
import { ApiError } from "~/services/http.client";
import { mountController } from "~/tests/mount-controller";
import { settle, signInAs } from "~/tests/people";

const routerPush = vi.hoisted(() => vi.fn());

vi.mock("vue-router", () => ({
  useRouter: () => ({ push: routerPush }),
}));

vi.mock("~/services/instance.api");
vi.mock("~/services/backups.api");
vi.mock("~/utils/download");

const mockStatus: instanceApi.StatusResponse = {
  version: "0.1.0",
  uptimeSeconds: 8100, // 2h 15m
  initializationState: "ready",
  databaseHealth: "healthy",
  keyAvailability: "available",
  projects: 3,
  environments: 6,
  secrets: 12,
  humanUsers: 2,
  aiAgents: 1,
  activeRunnerTokens: 2,
  activeAgentTokens: 1,
  backups: 4,
  observedAt: "2026-08-28T12:00:00Z",
};

async function mounted() {
  const { controller } = mountController(useInstanceController);
  await settle();
  return controller;
}

beforeEach(() => {
  setActivePinia(createPinia());
  vi.clearAllMocks();
  vi.mocked(instanceApi.fetchStatus).mockResolvedValue(mockStatus);
});

describe("useInstanceController", () => {
  it("loads status on mount and formats uptime and observed date", async () => {
    signInAs([]);
    const c = await mounted();

    expect(instanceApi.fetchStatus).toHaveBeenCalledTimes(1);
    expect(c.status.value).toEqual(mockStatus);
    expect(c.formattedUptime.value).toBe("2h 15m");
    expect(c.formattedObservedAt.value).toMatch(/\d/);
    expect(c.relativeObservedAt.value).not.toBe("");
    expect(c.relativeBootTime.value).not.toBe("");
  });

  it("lists only the counts the status API returns", async () => {
    signInAs([]);
    const c = await mounted();

    expect(c.counts.value).toEqual([
      { label: "Projects", value: 3 },
      { label: "Environments", value: 6 },
      { label: "Secrets", value: 12 },
      { label: "People", value: 2 },
      { label: "AI agents", value: 1 },
      { label: "Runner tokens", value: 2 },
      { label: "AI agent tokens", value: 1 },
      { label: "Backups", value: 4 },
    ]);
  });

  it.each([
    ["healthy", "ok", "Healthy", "ok"],
    ["available", "ok", "Available", "ok"],
    ["unhealthy", "crit", "Unhealthy", "down"],
    ["unavailable", "crit", "Unavailable", "down"],
    ["unknown", "neutral", "Unknown", "unknown"],
  ] as const)("shows %s as a %s stat", (value, tone, label, badge) => {
    signInAs([]);
    const { controller: c } = mountController(useInstanceController);
    expect(c.healthTone(value)).toBe(tone);
    expect(c.healthLabel(value)).toBe(label);
    expect(c.healthBadge(value)).toBe(badge);
  });

  it("handles a load error", async () => {
    signInAs([]);
    vi.mocked(instanceApi.fetchStatus).mockRejectedValueOnce(
      new Error("Network failed"),
    );
    const c = await mounted();

    expect(c.status.value).toBeNull();
    expect(c.loadError.value).toBe("Could not load the instance status.");
  });

  describe("master key download", () => {
    it("saves the key for root", async () => {
      signInAs([], { adminId: "usr_root", role: "root" });
      const key = new Blob(["key"]);
      vi.mocked(backupsApi.downloadMasterKeyBlob).mockResolvedValueOnce(key);
      const c = await mounted();

      await c.downloadMasterKey();

      expect(saveBlob).toHaveBeenCalledWith(key, "master.key");
      expect(c.masterKeyMessage.value?.tone).toBe("ok");
      expect(c.downloadingMasterKey.value).toBe(false);
    });

    it("is not offered to an admin, even one who can manage backups", async () => {
      signInAs(["instance.backup"]);
      const c = await mounted();

      expect(c.isRoot.value).toBe(false);
      await c.downloadMasterKey();

      expect(backupsApi.downloadMasterKeyBlob).not.toHaveBeenCalled();
      expect(saveBlob).not.toHaveBeenCalled();
    });

    it("says why the download failed", async () => {
      signInAs([], { adminId: "usr_root", role: "root" });
      vi.mocked(backupsApi.downloadMasterKeyBlob).mockRejectedValueOnce(
        new ApiError(500, {}, "The key could not be read."),
      );
      const c = await mounted();

      await c.downloadMasterKey();

      expect(saveBlob).not.toHaveBeenCalled();
      expect(c.masterKeyMessage.value).toEqual({
        text: "The key could not be read.",
        tone: "crit",
      });
    });
  });

  describe("factory reset", () => {
    it("stays off until the box is ticked, the phrase matches and a password is in", async () => {
      signInAs([], { adminId: "usr_root", role: "root" });
      const c = await mounted();

      expect(c.canReset.value).toBe(false);
      c.resetPassword.value = "rootpassword";
      c.resetConfirmation.value = "factory reset";
      c.resetAcknowledged.value = true;
      expect(c.canReset.value).toBe(false);

      await c.performReset();
      expect(instanceApi.factoryReset).not.toHaveBeenCalled();

      c.resetConfirmation.value = RESET_PHRASE;
      c.resetAcknowledged.value = false;
      expect(c.canReset.value).toBe(false);

      c.resetAcknowledged.value = true;
      expect(c.canReset.value).toBe(true);
    });

    it("resets, signs out locally and goes to setup", async () => {
      signInAs([], { adminId: "usr_root", role: "root" });
      vi.mocked(instanceApi.factoryReset).mockResolvedValueOnce();
      const c = await mounted();
      c.resetPassword.value = "rootpassword";
      c.resetConfirmation.value = RESET_PHRASE;
      c.resetAcknowledged.value = true;

      await c.performReset();

      expect(instanceApi.factoryReset).toHaveBeenCalledWith(
        "rootpassword",
        RESET_PHRASE,
        true,
      );
      expect(c.resetComplete.value).toBe(true);
      expect(routerPush).toHaveBeenCalledWith({ name: "setup" });
    });

    it("keeps the form when the server refuses", async () => {
      signInAs([], { adminId: "usr_root", role: "root" });
      vi.mocked(instanceApi.factoryReset).mockRejectedValueOnce(
        new ApiError(403, {}, "Wrong password."),
      );
      const c = await mounted();
      c.resetPassword.value = "wrong";
      c.resetConfirmation.value = RESET_PHRASE;
      c.resetAcknowledged.value = true;

      await c.performReset();

      expect(c.resetError.value).toBe(
        "Factory reset was not completed. Check the password and the confirmation.",
      );
      expect(routerPush).not.toHaveBeenCalled();
    });

    it("never runs for someone who isn't root", async () => {
      signInAs(["instance.backup"]);
      const c = await mounted();
      c.resetPassword.value = "rootpassword";
      c.resetConfirmation.value = RESET_PHRASE;
      c.resetAcknowledged.value = true;

      expect(c.canReset.value).toBe(false);
      await c.performReset();
      expect(instanceApi.factoryReset).not.toHaveBeenCalled();
    });
  });
});
