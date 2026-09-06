import { beforeEach, describe, expect, it, vi } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import { useInstanceController } from "./Instance.controller";
import * as instanceApi from "~/services/instance.api";

const routerPush = vi.hoisted(() => vi.fn());

vi.mock("vue-router", () => ({
  useRouter: () => ({ push: routerPush }),
}));

vi.mock("~/services/instance.api");

const mockStatus: instanceApi.StatusResponse = {
  version: "0.1.0",
  uptimeSeconds: 8100, // 2h 15m
  initializationState: "initialized",
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

beforeEach(() => {
  setActivePinia(createPinia());
  vi.clearAllMocks();
  vi.mocked(instanceApi.fetchStatus).mockResolvedValue(mockStatus);
});

describe("useInstanceController", () => {
  it("loads status and formats uptime and observed date", async () => {
    const c = useInstanceController();
    await c.load();

    expect(c.status.value).toEqual(mockStatus);
    expect(c.formattedUptime.value).toBe("2h 15m");
    expect(c.formattedObservedAt.value).toMatch(/\d/);
    expect(c.relativeObservedAt.value).not.toBe("");
    expect(c.relativeBootTime.value).not.toBe("");
  });

  it("handles health tones correctly", () => {
    const c = useInstanceController();
    expect(c.healthTone("healthy")).toBe("ok");
    expect(c.healthTone("available")).toBe("ok");
    expect(c.healthTone("unhealthy")).toBe("crit");
    expect(c.healthTone("unavailable")).toBe("crit");
    expect(c.healthTone("unknown")).toBe("neutral");
  });

  it("handles load error gracefully", async () => {
    vi.mocked(instanceApi.fetchStatus).mockRejectedValueOnce(new Error("Network failed"));
    const c = useInstanceController();
    await c.load();

    expect(c.status.value).toBeNull();
    expect(c.loadError.value).toBe("Could not load the instance status.");
  });

  it("performs factory reset and redirects to setup", async () => {
    vi.mocked(instanceApi.factoryReset).mockResolvedValueOnce();
    const c = useInstanceController();
    c.resetPassword.value = "rootpassword";
    c.resetConfirmation.value = "FACTORY RESET";
    c.resetAcknowledged.value = true;

    await c.performReset();

    expect(instanceApi.factoryReset).toHaveBeenCalledWith(
      "rootpassword",
      "FACTORY RESET",
      true,
    );
    expect(routerPush).toHaveBeenCalledWith({ name: "setup" });
  });
});
