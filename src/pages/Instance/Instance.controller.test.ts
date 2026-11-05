import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { freshPinia } from "~/tests/pinia";
import {
  POLL_INTERVAL_MS,
  RESET_PHRASE,
  useInstanceController,
} from "./Instance.controller";
import * as instanceApi from "~/services/instance.api";
import * as backupsApi from "~/services/backups.api";
import { saveBlob } from "~/utils/download";
import { copyText } from "~/utils/clipboard";
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
vi.mock("~/utils/clipboard");

const check = (
  id: string,
  status: instanceApi.CheckStatus,
  title: string,
  extra: Partial<instanceApi.StatusCheck> = {},
): instanceApi.StatusCheck => ({
  id,
  status,
  title,
  detail: `${title} detail`,
  value: null,
  fixPath: null,
  lastRunAt: "2026-08-28T11:59:50Z",
  ...extra,
});

const passingChecks = [
  check("database.ping", "ok", "Database answers", { value: "3 ms" }),
  check("master_key.file", "ok", "Master key matches the database"),
  check("audit.chain", "ok", "Audit log chain is intact", {
    value: "1,284 events",
  }),
];

/** State A: one critical, two warnings and a tip. */
const mockStatus: instanceApi.StatusResponse = {
  version: "0.1.0",
  uptimeSeconds: 8100, // 2h 15m
  initializationState: "ready",
  platform: { os: "linux", arch: "x86_64" },
  projects: 3,
  environments: 6,
  secrets: 12,
  humanUsers: 2,
  aiAgents: 1,
  activeRunnerTokens: 2,
  activeAgentTokens: 1,
  backups: 4,
  summary: {
    critical: 1,
    warnings: 2,
    tips: 1,
    passed: 3,
    checkedAt: "2026-08-28T11:59:50Z",
  },
  checks: [
    check("backup.age", "crit", "No backup in 12 days", {
      fixPath: "/backups",
    }),
    check("tokens.expiring", "warn", "3 tokens expire this week", {
      fixPath: "/projects/p/prj_1/e/env_1/tokens",
    }),
    check("backup.offsite", "warn", "The latest backup was never downloaded", {
      fixPath: "/backups",
    }),
    check("audit.retention", "tip", "The audit log has no size limit", {
      fixPath: "https://docs.dopbase.com/cli/serve#configuration-files",
    }),
    ...passingChecks,
  ],
  health: {
    database: { status: "ok", pingMs: 3.2, sizeBytes: 50_331_648 },
    masterKey: { status: "ok", file: "matches" },
    disk: { usedBytes: Math.round(2.1 * 1024 ** 3) },
    auditLog: {
      status: "ok",
      events: 1284,
      intact: true,
      checkedAt: "2026-08-28T11:58:00Z",
    },
  },
  activity: {
    hours: [],
    secretReads: {
      total: 1204,
      hourly: Array.from({ length: 24 }, (_, hour) => hour),
      runnerTokens: 3,
      people: 2,
      aiAgents: 0,
    },
    failedSignIns: {
      total: 3,
      hourly: [],
      accounts: ["root@example.com"],
      addresses: ["203.0.113.7"],
    },
    rateLimited: { total: 0, hourly: [] },
    activeSessions: { browser: 2, cli: 2 },
  },
  latestRelease: {
    version: "v0.3.1",
    updateAvailable: true,
    url: "https://github.com/dopbase/dopbase/releases/tag/v0.3.1",
    checkedAt: "2026-08-28T06:00:00Z",
  },
  observedAt: "2026-08-28T12:00:00Z",
};

/** State B: every check passes. */
const allGood: instanceApi.StatusResponse = {
  ...mockStatus,
  summary: { ...mockStatus.summary!, critical: 0, warnings: 0, tips: 0 },
  checks: passingChecks,
  latestRelease: { ...mockStatus.latestRelease!, updateAvailable: false },
};

/** State C: the key file on disk changed. */
const keyChanged: instanceApi.StatusResponse = {
  ...mockStatus,
  summary: { ...mockStatus.summary!, critical: 1, warnings: 0, tips: 0 },
  checks: [
    check(
      "master_key.file",
      "crit",
      "The key file no longer matches the key in use",
      { fixPath: "/backups" },
    ),
    ...passingChecks.filter((item) => item.id !== "master_key.file"),
  ],
  health: {
    ...mockStatus.health!,
    masterKey: { status: "crit", file: "changed" },
  },
};

async function mounted() {
  const { controller } = mountController(useInstanceController);
  await settle();
  return controller;
}

beforeEach(() => {
  freshPinia();
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

  it("shows the last results at once when you come back to Status", async () => {
    signInAs([]);
    const first = mountController(useInstanceController);
    await settle();
    first.unmount();

    const { controller: again } = mountController(useInstanceController);

    expect(again.status.value).toEqual(mockStatus);
    expect(again.loading.value).toBe(false);
    await settle();
    expect(instanceApi.fetchStatus).toHaveBeenCalledTimes(1);
  });

  it("shows a dash for a count the server couldn't read", async () => {
    signInAs([]);
    vi.mocked(instanceApi.fetchStatus).mockResolvedValueOnce({
      ...mockStatus,
      activeAgentTokens: null,
    });
    const c = await mounted();

    expect(
      c.counts.value.find((item) => item.label === "AI agent tokens")?.value,
    ).toBe("—");
  });

  it("lists only the counts the status API returns", async () => {
    signInAs([]);
    const c = await mounted();

    expect(
      c.counts.value.map(({ label, value }) => ({ label, value })),
    ).toEqual([
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

  it("explains the account and token counts", async () => {
    signInAs([]);
    const c = await mounted();

    expect(
      c.counts.value.filter((item) => item.hint).map((item) => item.label),
    ).toEqual(["People", "AI agents", "Runner tokens", "AI agent tokens"]);
  });

  describe("banner", () => {
    it("counts what needs attention and offers the new release", async () => {
      signInAs(["instance.view"]);
      const c = await mounted();

      expect(c.banner.value).toMatchObject({
        tone: "crit",
        title: "4 things need attention",
        keyProblem: false,
      });
      expect(c.banner.value?.detail).toMatch(
        /^1 critical, 2 warnings and 1 tip\. 3 other checks passed\. Checked /,
      );
      expect(c.serverLine.value).toBe("Dopbase 0.1.0 · up 2h 15m");
      expect(c.releaseBadge.value).toEqual({
        tone: "accent",
        text: "0.3.1 is out",
      });
    });

    it("is calm when every check passes", async () => {
      signInAs(["instance.view"]);
      vi.mocked(instanceApi.fetchStatus).mockResolvedValue(allGood);
      const c = await mounted();

      expect(c.banner.value).toMatchObject({
        tone: "ok",
        title: "Everything looks good",
      });
      expect(c.banner.value?.detail).toMatch(/^All 3 checks passed\. /);
      expect(c.releaseBadge.value).toEqual({
        tone: "ok",
        text: "latest version",
      });
    });

    it("leads with the key file when it no longer matches", async () => {
      signInAs(["instance.view"]);
      vi.mocked(instanceApi.fetchStatus).mockResolvedValue(keyChanged);
      const c = await mounted();

      expect(c.banner.value).toMatchObject({
        tone: "crit",
        title: "The master key file changed on disk",
        keyProblem: true,
      });
      expect(c.healthCards.value[1]).toMatchObject({
        value: "File changed",
        badge: { tone: "crit", text: "at risk" },
        detail: "Key in use still works",
      });
    });

    it("is left out for viewers without instance.view", async () => {
      signInAs([]);
      vi.mocked(instanceApi.fetchStatus).mockResolvedValue({
        ...mockStatus,
        summary: null,
        checks: null,
        health: null,
        activity: null,
        latestRelease: null,
      });
      const c = await mounted();

      expect(c.banner.value).toBeNull();
      expect(c.hasChecks.value).toBe(false);
      expect(c.healthCards.value).toEqual([]);
      expect(c.activityTiles.value).toEqual([]);
    });
  });

  it("reads the four health cards", async () => {
    signInAs(["instance.view"]);
    const c = await mounted();

    expect(
      c.healthCards.value.map(({ label, value, badge, detail }) => ({
        label,
        value,
        badge: badge?.text,
        detail,
      })),
    ).toEqual([
      {
        label: "Database",
        value: "Healthy",
        badge: "ok",
        detail: "Latency under 5 ms · 48 MB",
      },
      {
        label: "Master key",
        value: "Verified",
        badge: "ok",
        detail: "Matches this database",
      },
      {
        label: "Disk usage",
        value: "2.1 GB",
        badge: undefined,
        detail: "Storage used",
      },
      {
        label: "Audit log",
        value: "Intact",
        badge: "ok",
        detail: expect.stringMatching(/^Chain checked /),
      },
    ]);
  });

  it("says Unknown when the data directory can't be read", async () => {
    signInAs(["instance.view"]);
    vi.mocked(instanceApi.fetchStatus).mockResolvedValue({
      ...mockStatus,
      health: { ...mockStatus.health!, disk: null },
    });
    const c = await mounted();

    const disk = c.healthCards.value.find((card) => card.id === "disk");
    expect(disk).toMatchObject({
      value: "Unknown",
      detail: "Couldn't read the data directory",
    });
    expect(disk?.badge).toBeUndefined();
  });

  it("gives the exact latency once the database takes 5 ms or more", async () => {
    signInAs(["instance.view"]);
    vi.mocked(instanceApi.fetchStatus).mockResolvedValue({
      ...mockStatus,
      health: {
        ...mockStatus.health!,
        database: { status: "warn", pingMs: 12.4, sizeBytes: 50_331_648 },
      },
    });
    const c = await mounted();

    const database = c.healthCards.value.find((card) => card.id === "database");
    expect(database?.detail).toBe("Latency 12 ms · 48 MB");
  });

  it("sums up the last 24 hours", async () => {
    signInAs(["instance.view"]);
    const c = await mounted();

    expect(
      c.activityTiles.value.map(({ label, value, detail }) => ({
        label,
        value,
        detail,
      })),
    ).toEqual([
      {
        label: "Secret reads",
        value: "1,204",
        detail: "By 3 runner tokens and 2 people",
      },
      {
        label: "Failed sign-ins",
        value: "3",
        detail: "All for root@example.com, from 203.0.113.7",
      },
      {
        label: "Rate limited",
        value: "0",
        detail: "No client was slowed down",
      },
      {
        label: "Active sessions",
        value: "4",
        detail: "2 browser sessions, 2 CLI sessions",
      },
    ]);
    expect(c.activityTiles.value[0].hourly).toHaveLength(24);
  });

  it("names AI agents that read secrets apart from people", async () => {
    signInAs(["instance.view"]);
    vi.mocked(instanceApi.fetchStatus).mockResolvedValue({
      ...mockStatus,
      activity: {
        ...mockStatus.activity!,
        secretReads: {
          total: 9,
          hourly: [],
          runnerTokens: 0,
          people: 0,
          aiAgents: 1,
        },
      },
    });
    const c = await mounted();

    expect(c.activityTiles.value[0].detail).toBe("By 1 AI agent");
  });

  it("splits failing checks from passing ones and folds the passing", async () => {
    signInAs(["instance.view"]);
    const c = await mounted();

    expect(c.attention.value.map((item) => item.id)).toEqual([
      "backup.age",
      "tokens.expiring",
      "backup.offsite",
      "audit.retention",
    ]);
    expect(c.passed.value).toHaveLength(3);
    expect(c.showPassed.value).toBe(false);
    expect(c.severityLabel("crit")).toBe("Critical");
    expect(c.severityTone("tip")).toBe("neutral");
  });

  describe("fix actions", () => {
    it("hide fixes the viewer can't use", async () => {
      signInAs(["instance.view"]);
      const c = await mounted();
      const [backup, tokens, offsite, retention] = c.attention.value;

      expect(c.fixAction(backup)).toBeNull();
      expect(c.fixAction(offsite)).toBeNull();
      expect(c.fixAction(tokens)).toMatchObject({
        label: "Review tokens",
        kind: "route",
        to: "/projects/p/prj_1/e/env_1/tokens",
      });
      expect(c.fixAction(retention)).toMatchObject({
        label: "How to set it",
        kind: "external",
      });
    });

    it("go to the page that fixes the check", async () => {
      signInAs(["instance.view", "instance.backup"]);
      const c = await mounted();
      const [backup] = c.attention.value;

      expect(c.fixAction(backup)).toMatchObject({
        label: "Create backup",
        to: "/backups",
      });
      await c.runFix(backup);
      expect(routerPush).toHaveBeenCalledWith("/backups");
    });

    it("download the key in use for root only", async () => {
      signInAs(["instance.view", "instance.backup"]);
      vi.mocked(instanceApi.fetchStatus).mockResolvedValue(keyChanged);
      const admin = await mounted();
      expect(admin.fixAction(admin.attention.value[0])).toBeNull();

      freshPinia();
      signInAs([], { adminId: "usr_root", role: "root" });
      const key = new Blob(["key"]);
      vi.mocked(backupsApi.downloadMasterKeyBlob).mockResolvedValueOnce(key);
      const c = await mounted();
      const [keyCheck] = c.attention.value;

      expect(c.fixAction(keyCheck)).toMatchObject({
        label: "Download key in use",
        kind: "master-key",
      });
      await c.runFix(keyCheck);
      expect(saveBlob).toHaveBeenCalledWith(key, "master.key");
    });
  });

  describe("polling", () => {
    beforeEach(() => vi.useFakeTimers());
    afterEach(() => vi.useRealTimers());

    it("asks again every 30 seconds", async () => {
      signInAs(["instance.view"]);
      mountController(useInstanceController);
      await vi.advanceTimersByTimeAsync(0);
      expect(instanceApi.fetchStatus).toHaveBeenCalledTimes(1);

      await vi.advanceTimersByTimeAsync(POLL_INTERVAL_MS);
      expect(instanceApi.fetchStatus).toHaveBeenCalledTimes(2);
    });

    it("keeps the last results when a poll fails", async () => {
      signInAs(["instance.view"]);
      const { controller: c } = mountController(useInstanceController);
      await vi.advanceTimersByTimeAsync(0);
      vi.mocked(instanceApi.fetchStatus).mockRejectedValueOnce(
        new Error("Network failed"),
      );

      await vi.advanceTimersByTimeAsync(POLL_INTERVAL_MS);

      expect(c.status.value).toEqual(mockStatus);
      expect(c.loadError.value).toBeNull();
      expect(c.staleSince.value).toBe(mockStatus.observedAt);
    });

    it("drops a poll that was already out when Run checks was pressed", async () => {
      signInAs(["instance.view"]);
      const { controller: c } = mountController(useInstanceController);
      await vi.advanceTimersByTimeAsync(0);
      let answerPoll!: (status: instanceApi.StatusResponse) => void;
      vi.mocked(instanceApi.fetchStatus).mockReturnValueOnce(
        new Promise((resolve) => (answerPoll = resolve)),
      );
      await vi.advanceTimersByTimeAsync(POLL_INTERVAL_MS);
      expect(c.loading.value).toBe(true);

      vi.mocked(instanceApi.runStatusChecks).mockResolvedValueOnce(allGood);
      await c.runChecks();
      answerPoll(mockStatus);
      await vi.advanceTimersByTimeAsync(0);

      expect(c.status.value).toEqual(allGood);
      expect(c.loading.value).toBe(false);
    });

    it("keeps a run error when an older poll answers after it", async () => {
      signInAs(["instance.view"]);
      const { controller: c } = mountController(useInstanceController);
      await vi.advanceTimersByTimeAsync(0);
      let answerPoll!: (status: instanceApi.StatusResponse) => void;
      vi.mocked(instanceApi.fetchStatus).mockReturnValueOnce(
        new Promise((resolve) => (answerPoll = resolve)),
      );
      await vi.advanceTimersByTimeAsync(POLL_INTERVAL_MS);

      vi.mocked(instanceApi.runStatusChecks).mockRejectedValueOnce(
        new ApiError(429, {}, "Too many requests."),
      );
      await c.runChecks();
      answerPoll(allGood);
      await vi.advanceTimersByTimeAsync(0);

      expect(c.runError.value).toBe(
        "Checks just ran. Wait a few seconds and try again.",
      );
      expect(c.status.value).toEqual(mockStatus);
    });

    it("stops when the page closes", async () => {
      signInAs(["instance.view"]);
      const { unmount } = mountController(useInstanceController);
      await vi.advanceTimersByTimeAsync(0);
      unmount();

      await vi.advanceTimersByTimeAsync(POLL_INTERVAL_MS * 2);
      expect(instanceApi.fetchStatus).toHaveBeenCalledTimes(1);
    });
  });

  describe("run checks", () => {
    it("shows the fresh results", async () => {
      signInAs(["instance.view"]);
      vi.mocked(instanceApi.runStatusChecks).mockResolvedValueOnce(allGood);
      const c = await mounted();

      await c.runChecks();

      expect(c.status.value).toEqual(allGood);
      expect(c.running.value).toBe(false);
      expect(c.runError.value).toBeNull();
    });

    it("explains the wait when checks just ran", async () => {
      signInAs(["instance.view"]);
      vi.mocked(instanceApi.runStatusChecks).mockRejectedValueOnce(
        new ApiError(429, {}, "Too many requests."),
      );
      const c = await mounted();

      await c.runChecks();

      expect(c.runError.value).toBe(
        "Checks just ran. Wait a few seconds and try again.",
      );
      expect(c.status.value).toEqual(mockStatus);
    });
  });

  it("copies a plain-text report of what the page shows", async () => {
    signInAs(["instance.view"]);
    vi.mocked(copyText).mockResolvedValueOnce(true);
    const c = await mounted();

    const report = c.diagnosticsText();
    expect(report).toContain("Dopbase 0.1.0 on linux/x86_64, up 2h 15m");
    expect(report).toContain(
      "Checks: 1 critical, 2 warnings, 1 tips, 3 passed",
    );
    expect(report).toContain(
      "[crit] No backup in 12 days: No backup in 12 days detail",
    );
    expect(report).toContain("[ok] Database answers (3 ms)");
    expect(report).toContain("1204 secret reads, 3 failed sign-ins");

    await c.copyDiagnostics();
    expect(copyText).toHaveBeenCalledWith(report);
    expect(c.diagnosticsCopied.value).toBe(true);
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
    it("starts folded", async () => {
      signInAs([], { adminId: "usr_root", role: "root" });
      const c = await mounted();
      expect(c.resetOpen.value).toBe(false);
    });

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
