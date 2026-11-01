/* eslint-disable vue/one-component-per-file */
import { createApp, defineComponent, h } from "vue";
import { createPinia, setActivePinia } from "pinia";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import * as instanceApi from "~/services/instance.api";
import { settle, signInAs } from "~/tests/people";

vi.mock("vue-router", () => ({
  useRouter: () => ({ push: vi.fn() }),
}));
vi.mock("~/services/instance.api");
vi.mock("~/services/backups.api");
vi.mock("~/layouts", async () => {
  const { defineComponent: define, h: render } = await import("vue");
  return {
    DashboardLayout: define({
      setup(_, { slots }) {
        return () => render("div", slots.default?.());
      },
    }),
  };
});

import InstancePage from "./Instance.page.vue";

const ok = (id: string, title: string, value: string | null = null) => ({
  id,
  status: "ok" as const,
  title,
  detail: `${title}.`,
  value,
  fixPath: null,
  lastRunAt: "2026-08-28T11:59:50Z",
});

const passing = [
  ok("database.ping", "Database answers", "3 ms"),
  ok("master_key.file", "Master key matches the database"),
  ok("audit.chain", "Audit log chain is intact", "1,284 events"),
];

const base: instanceApi.StatusResponse = {
  version: "0.2.0",
  uptimeSeconds: 532_800,
  initializationState: "ready",
  platform: { os: "linux", arch: "x86_64" },
  projects: 4,
  environments: 9,
  secrets: 51,
  humanUsers: 7,
  aiAgents: 3,
  activeRunnerTokens: 3,
  activeAgentTokens: 5,
  backups: 4,
  summary: {
    critical: 0,
    warnings: 0,
    tips: 0,
    passed: 3,
    checkedAt: new Date().toISOString(),
  },
  checks: passing,
  health: {
    database: { status: "ok", pingMs: 3, sizeBytes: 50_331_648 },
    masterKey: { status: "ok", file: "matches" },
    disk: {
      status: "ok",
      freeBytes: 18 * 1024 ** 3,
      totalBytes: 64 * 1024 ** 3,
    },
    auditLog: {
      status: "ok",
      events: 1284,
      intact: true,
      checkedAt: new Date().toISOString(),
    },
  },
  activity: {
    hours: [],
    secretReads: {
      total: 1204,
      hourly: Array(24).fill(2),
      runnerTokens: 3,
      people: 2,
      aiAgents: 0,
    },
    failedSignIns: { total: 0, hourly: [], accounts: [], addresses: [] },
    rateLimited: { total: 0, hourly: [] },
    activeSessions: { browser: 2, cli: 2 },
  },
  latestRelease: null,
  observedAt: new Date().toISOString(),
};

const needsAttention: instanceApi.StatusResponse = {
  ...base,
  summary: { ...base.summary!, critical: 1, tips: 1 },
  checks: [
    {
      ...ok("backup.age", "No backup in 12 days"),
      status: "crit",
      fixPath: "/backups",
    },
    {
      ...ok("audit.retention", "The audit log has no size limit"),
      status: "tip",
      fixPath: "https://docs.dopbase.com/cli/serve#configuration-files",
    },
    ...passing,
  ],
};

const keyChanged: instanceApi.StatusResponse = {
  ...base,
  summary: { ...base.summary!, critical: 1 },
  checks: [
    {
      ...ok("master_key.file", "The key file no longer matches the key in use"),
      status: "crit",
      fixPath: "/backups",
    },
    ...passing.filter((check) => check.id !== "master_key.file"),
  ],
  health: { ...base.health!, masterKey: { status: "crit", file: "changed" } },
};

let app: ReturnType<typeof createApp> | null = null;

async function mountPage(
  status: instanceApi.StatusResponse,
): Promise<HTMLElement> {
  vi.mocked(instanceApi.fetchStatus).mockResolvedValue(status);
  const host = document.createElement("div");
  document.body.append(host);
  app = createApp({ render: () => h(InstancePage) });
  app.component(
    "RouterLink",
    defineComponent({
      props: { to: { type: [Object, String], required: true } },
      setup(_, { slots, attrs }) {
        return () => h("a", { ...attrs, href: "#" }, slots.default?.());
      },
    }),
  );
  app.mount(host);
  await settle();
  return host;
}

function find(host: HTMLElement, id: string): HTMLElement | null {
  return host.querySelector(`[data-testid="${id}"]`);
}

beforeEach(() => {
  setActivePinia(createPinia());
});

afterEach(() => {
  app?.unmount();
  app = null;
  document.body.innerHTML = "";
  vi.clearAllMocks();
});

describe("Status page", () => {
  it("lists what needs attention and folds the passing checks", async () => {
    signInAs(["instance.view"]);
    const host = await mountPage(needsAttention);

    const banner = find(host, "status-banner")!;
    expect(banner.dataset.tone).toBe("crit");
    expect(banner.textContent).toContain("2 things need attention");
    expect(banner.textContent).toContain("Dopbase 0.2.0 · up 6d 4h");

    const checks = find(host, "checks")!;
    expect(checks.textContent).toContain("Needs attention");
    expect(find(host, "check-backup.age")?.textContent).toContain("Critical");
    // Backups need instance.backup, so that fix is hidden; docs links stay.
    expect(find(host, "fix-backup.age")).toBeNull();
    expect(find(host, "fix-audit.retention")?.textContent).toContain(
      "How to set it",
    );
    expect(checks.textContent).toContain("3 checks passed");
    expect(find(host, "passed-database.ping")).toBeNull();

    find(host, "toggle-passed")!.click();
    await settle();
    expect(find(host, "passed-database.ping")?.textContent).toContain("3 ms");
  });

  it("shows the fix to someone who can use it", async () => {
    signInAs(["instance.view", "instance.backup"]);
    const host = await mountPage(needsAttention);

    expect(find(host, "fix-backup.age")?.textContent).toContain(
      "Create backup",
    );
  });

  it("lists every check when all pass", async () => {
    signInAs(["instance.view"]);
    const host = await mountPage(base);

    expect(find(host, "status-banner")?.textContent).toContain(
      "Everything looks good",
    );
    expect(find(host, "checks")?.textContent).toContain("Checks");
    expect(find(host, "passed-audit.chain")?.textContent).toContain(
      "1,284 events",
    );
    expect(find(host, "toggle-passed")).toBeNull();
    expect(find(host, "health-database")?.textContent).toContain("Healthy");
    expect(find(host, "activity-secretReads")?.textContent).toContain("1,204");
  });

  it("leads with the key file and offers root the key in use", async () => {
    signInAs([], { adminId: "usr_root", role: "root" });
    const host = await mountPage(keyChanged);

    const banner = find(host, "status-banner")!;
    expect(banner.textContent).toContain("The master key file changed on disk");
    expect(banner.textContent).toContain("How to fix it");
    expect(find(host, "health-masterKey")?.textContent).toContain(
      "File changed",
    );
    expect(find(host, "fix-master_key.file")?.textContent).toContain(
      "Download key in use",
    );
  });

  it("keeps the factory reset folded until asked", async () => {
    signInAs([], { adminId: "usr_root", role: "root" });
    const host = await mountPage(base);

    expect(find(host, "factory-reset")?.textContent).toContain(
      "Show reset steps",
    );
    expect(find(host, "factory-reset-submit")).toBeNull();

    find(host, "toggle-reset")!.click();
    await settle();
    expect(find(host, "factory-reset-submit")).not.toBeNull();
  });

  it("leaves What's stored as it was", async () => {
    signInAs(["instance.view"]);
    const host = await mountPage(base);

    const stored = find(host, "instance-content")!;
    expect(stored.textContent).toContain("What's stored");
    expect(stored.textContent).toContain("Counts across the whole instance.");
    expect(stored.textContent).toContain("AI agent tokens");
  });
});
