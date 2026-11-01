import { beforeEach, describe, expect, it, vi } from "vitest";
import { ref } from "vue";
import { useProjectSetupController } from "./ProjectSetup.controller";
import * as environmentsApi from "~/services/environments.api";
import type { Environment } from "~/services";
import { mountController } from "~/tests/mount-controller";

vi.mock("~/services/environments.api");

function environment(name: string): Environment {
  return {
    id: `env_${name}`,
    projectId: "prj_pay",
    projectName: "payment-service",
    name,
    createdAt: "",
    updatedAt: "",
  };
}

const onDone = vi.fn();

beforeEach(() => {
  vi.resetAllMocks();
  vi.mocked(environmentsApi.createEnvironment).mockImplementation(
    async (_project, name) => environment(name),
  );
  vi.mocked(environmentsApi.setEnvironmentProtected).mockImplementation(
    async (id) => ({ ...environment(id.slice(4)), protected: true }),
  );
});

function mount() {
  return mountController(() =>
    useProjectSetupController(ref("payment-service"), onDone),
  ).controller;
}

function row(c: ReturnType<typeof mount>, name: string) {
  return c.rows.value.find((candidate) => candidate.name === name)!;
}

describe("useProjectSetupController", () => {
  it("starts from development, staging and a protected production", () => {
    const c = mount();

    expect(c.rows.value.map((r) => [r.name, r.selected, r.protected])).toEqual([
      ["development", true, false],
      ["staging", true, false],
      ["production", true, true],
    ]);
    expect(c.submitLabel.value).toBe("Create 3 environments");
    expect(c.firstName.value).toBe("development");
  });

  it("counts only the ticked ones and won't create none", async () => {
    const c = mount();
    row(c, "staging").selected = false;
    expect(c.submitLabel.value).toBe("Create 2 environments");

    row(c, "development").selected = false;
    expect(c.submitLabel.value).toBe("Create 1 environment");
    expect(c.firstName.value).toBe("production");

    row(c, "production").selected = false;
    expect(c.canCreate.value).toBe(false);
    await c.create();
    expect(environmentsApi.createEnvironment).not.toHaveBeenCalled();
  });

  it("adds another name only when the server would accept it", () => {
    const c = mount();

    c.editNewName("QA env");
    expect(c.addRow()).toBe(false);
    expect(c.newNameError.value).toMatch(/lowercase letters/);

    c.editNewName("staging");
    expect(c.addRow()).toBe(false);
    expect(c.newNameError.value).toBe("staging is already on the list.");

    c.editNewName("a".repeat(64));
    expect(c.addRow()).toBe(false);
    expect(c.newNameError.value).toMatch(/63 characters/);

    c.editNewName(" qa ");
    expect(c.newNameError.value).toBeNull();
    expect(c.addRow()).toBe(true);
    expect(row(c, "qa")).toMatchObject({ selected: true, custom: true });
    expect(c.newName.value).toBe("");

    c.removeRow("qa");
    c.removeRow("staging");
    expect(c.rows.value.map((r) => r.name)).toEqual([
      "development",
      "staging",
      "production",
    ]);
  });

  it("creates the ticked ones in order, protects the marked, then opens the first", async () => {
    const c = mount();
    row(c, "staging").selected = false;

    await c.create();

    expect(vi.mocked(environmentsApi.createEnvironment).mock.calls).toEqual([
      ["payment-service", "development"],
      ["payment-service", "production"],
    ]);
    expect(environmentsApi.setEnvironmentProtected).toHaveBeenCalledOnce();
    expect(environmentsApi.setEnvironmentProtected).toHaveBeenCalledWith(
      "env_production",
      true,
    );
    expect(onDone).toHaveBeenCalledWith("development");
  });

  it("keeps what worked when one fails, and retries only the rest", async () => {
    vi.mocked(environmentsApi.createEnvironment).mockImplementation(
      async (_project, name) => {
        if (name === "staging") throw new Error("Too many environments.");
        return environment(name);
      },
    );
    const c = mount();

    await c.create();

    expect(row(c, "development").status).toBe("created");
    expect(row(c, "staging")).toMatchObject({
      status: "failed",
      error: "Too many environments.",
    });
    // It stops at the failure, so production is still to do.
    expect(row(c, "production").status).toBe("idle");
    expect(onDone).not.toHaveBeenCalled();
    expect(c.submitLabel.value).toBe("Create 2 environments");

    vi.mocked(environmentsApi.createEnvironment).mockClear();
    row(c, "staging").selected = false;
    await c.create();

    expect(vi.mocked(environmentsApi.createEnvironment).mock.calls).toEqual([
      ["payment-service", "production"],
    ]);
    expect(onDone).toHaveBeenCalledWith("development");
  });

  it("opens what was created without retrying", async () => {
    vi.mocked(environmentsApi.createEnvironment).mockImplementation(
      async (_project, name) => {
        if (name === "staging") throw new Error("Nope.");
        return environment(name);
      },
    );
    const c = mount();
    await c.create();

    await c.openCreated();

    expect(onDone).toHaveBeenCalledWith("development");
  });

  it("stays put when it couldn't protect one, and retries only the protection", async () => {
    vi.mocked(environmentsApi.setEnvironmentProtected).mockRejectedValueOnce(
      new Error("Forbidden"),
    );
    const c = mount();

    await c.create();

    expect(row(c, "production")).toMatchObject({
      status: "created",
      unprotected: true,
      error:
        "Created, but not protected yet. Try again, or turn it on in Settings.",
    });
    // Leaving now would hide the warning with production open to everyone.
    expect(onDone).not.toHaveBeenCalled();
    expect(c.stopped.value).toBe(true);
    expect(c.submitLabel.value).toBe("Try again");

    vi.mocked(environmentsApi.createEnvironment).mockClear();
    await c.create();

    expect(environmentsApi.createEnvironment).not.toHaveBeenCalled();
    expect(environmentsApi.setEnvironmentProtected).toHaveBeenLastCalledWith(
      "env_production",
      true,
    );
    expect(row(c, "production")).toMatchObject({
      unprotected: false,
      error: null,
    });
    expect(onDone).toHaveBeenCalledWith("development");
  });

  it("stops creating the rest after a protect failure", async () => {
    vi.mocked(environmentsApi.setEnvironmentProtected).mockRejectedValue(
      new Error("Forbidden"),
    );
    const c = mount();
    row(c, "staging").protected = true;

    await c.create();

    expect(row(c, "staging").unprotected).toBe(true);
    expect(row(c, "production").status).toBe("idle");
    expect(onDone).not.toHaveBeenCalled();

    // Going on without protection is still up to them.
    await c.openCreated();
    expect(onDone).toHaveBeenCalledWith("development");
  });
});
