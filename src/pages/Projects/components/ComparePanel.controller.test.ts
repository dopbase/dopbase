import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { nextTick, ref } from "vue";
import { useComparePanelController } from "./ComparePanel.controller";
import * as compareApi from "~/services/compare.api";
import type { CompareResult } from "~/services/compare.api";
import * as secretsApi from "~/services/secrets.api";
import { ApiError } from "~/services/http.client";
import type { Environment } from "~/services";
import { mountController } from "~/tests/mount-controller";

vi.mock("~/services/secrets.api", async (original) => ({
  ...(await original<typeof secretsApi>()),
  setSecret: vi.fn(),
}));
vi.mock("~/services/compare.api", async (original) => ({
  ...(await original<typeof compareApi>()),
  compareEnvironments: vi.fn(),
  copySecret: vi.fn(),
}));

function environment(
  id: string,
  name: string,
  extra: Partial<Environment> = {},
): Environment {
  return {
    id,
    projectId: "prj_pay",
    projectName: "payments",
    name,
    level: "manage",
    createdAt: "",
    updatedAt: "",
    ...extra,
  };
}

const prod = environment("env_prod", "prod", { protected: true });
const staging = environment("env_staging", "staging");
const dev = environment("env_dev", "dev");
const qa = environment("env_qa", "qa");
const uat = environment("env_uat", "uat");

const column = (env: Environment, extra = {}) => ({
  id: env.id,
  name: env.name,
  protected: env.protected ?? false,
  level: "manage" as const,
  valuesCompared: true,
  ...extra,
});

/** prod as the base, against staging and dev. */
const result: CompareResult = {
  base: column(prod),
  environments: [column(staging), column(dev)],
  keys: [
    {
      key: "FEATURE_FLAGS",
      base: null,
      others: [
        { version: 3, sameAsBase: null },
        { version: 5, sameAsBase: null },
      ],
    },
    {
      key: "SENTRY_DSN",
      base: 2,
      others: [
        { version: null, sameAsBase: null },
        { version: null, sameAsBase: null },
      ],
    },
    {
      key: "STRIPE_KEY",
      base: 4,
      others: [
        { version: 2, sameAsBase: false },
        { version: 7, sameAsBase: true },
      ],
    },
    {
      key: "API_URL",
      base: 1,
      others: [
        { version: 1, sameAsBase: true },
        { version: 1, sameAsBase: true },
      ],
    },
  ],
};

function panel(base = prod, list = [prod, staging, dev]) {
  const baseRef = ref(base);
  const listRef = ref(list);
  const { controller } = mountController(() =>
    useComparePanelController(baseRef, listRef),
  );
  return { c: controller, baseRef, listRef };
}

beforeEach(() => {
  localStorage.clear();
  vi.mocked(compareApi.compareEnvironments).mockResolvedValue(result);
  vi.mocked(compareApi.copySecret).mockResolvedValue({
    key: "FEATURE_FLAGS",
    version: 1,
    createdAt: "",
    updatedAt: "",
  });
});

afterEach(() => {
  vi.clearAllMocks();
});

describe("useComparePanelController — which environments", () => {
  it("starts with one environment and takes at most three", async () => {
    const { c } = panel(prod, [prod, staging, dev, qa, uat]);

    expect(c.selected.value.map((env) => env.name)).toEqual(["staging"]);
    expect(compareApi.compareEnvironments).toHaveBeenCalledWith(
      "env_prod",
      ["env_staging"],
      expect.any(AbortSignal),
    );

    c.add("env_dev");
    c.add("env_qa");
    expect(c.selected.value.map((env) => env.name)).toEqual([
      "staging",
      "dev",
      "qa",
    ]);
    expect(c.canAdd.value).toBe(false);
    c.add("env_uat");
    expect(c.selected.value).toHaveLength(3);

    c.remove("env_dev");
    await nextTick();
    expect(c.canAdd.value).toBe(true);
    expect(c.addable.value.map((env) => env.name)).toEqual(["dev", "uat"]);
  });

  it("remembers the choice per project", async () => {
    const first = panel();
    first.c.add("env_dev");
    first.c.remove("env_staging");
    await nextTick();

    const { c } = panel();
    expect(c.selected.value.map((env) => env.name)).toEqual(["dev"]);
  });

  it("swaps the old base in when the base changes", async () => {
    const { c, baseRef } = panel();
    c.add("env_dev");
    await vi.waitFor(() => expect(c.result.value).not.toBeNull());
    vi.mocked(compareApi.compareEnvironments).mockReturnValueOnce(
      new Promise(() => {}),
    );

    baseRef.value = staging;
    await nextTick();

    expect(c.selected.value.map((env) => env.name)).toEqual(["prod", "dev"]);
    // The old table read against prod, so it goes until staging's arrives.
    expect(c.result.value).toBeNull();
    await vi.waitFor(() =>
      expect(compareApi.compareEnvironments).toHaveBeenLastCalledWith(
        "env_staging",
        ["env_prod", "env_dev"],
        expect.any(AbortSignal),
      ),
    );
  });
});

describe("useComparePanelController — rows", () => {
  it("reads every cell against the base and lists differences first", async () => {
    const { c } = panel();
    await vi.waitFor(() => expect(c.rows.value).toHaveLength(4));

    const statuses = Object.fromEntries(
      c.rows.value.map((row) => [
        row.key,
        [row.base.status, ...row.cells.map((cell) => cell.status)],
      ]),
    );
    expect(statuses).toEqual({
      FEATURE_FLAGS: ["base-missing", "set", "set"],
      SENTRY_DSN: ["base-set", "missing", "missing"],
      STRIPE_KEY: ["base-set", "differs", "same"],
      API_URL: ["base-set", "same", "same"],
    });
    expect(c.visibleRows.value.map((row) => row.key)).toEqual([
      "FEATURE_FLAGS",
      "SENTRY_DSN",
      "STRIPE_KEY",
    ]);
    expect(c.summary.value).toEqual({
      missingInBase: 1,
      missingElsewhere: 1,
      differs: 1,
      same: 1,
    });

    c.view.value = "all";
    c.filter.value = "api";
    expect(c.visibleRows.value.map((row) => row.key)).toEqual(["API_URL"]);
  });

  it("lets you fill Not set cells where you can write", async () => {
    vi.mocked(compareApi.compareEnvironments).mockResolvedValue({
      ...result,
      environments: [column(staging), column(dev, { level: "read" })],
    });
    const { c } = panel();
    await vi.waitFor(() => expect(c.rows.value).toHaveLength(4));
    const row = (key: string) => c.rows.value.find((r) => r.key === key)!;

    expect(row("FEATURE_FLAGS").base.fillable).toBe(true);
    expect(row("SENTRY_DSN").cells.map((cell) => cell.fillable)).toEqual([
      true,
      false,
    ]);
    // Set and Differs cells are never fillable.
    expect(row("STRIPE_KEY").cells.some((cell) => cell.fillable)).toBe(false);
  });

  it("marks names-only columns as not compared", async () => {
    vi.mocked(compareApi.compareEnvironments).mockResolvedValue({
      ...result,
      environments: [
        column(staging),
        column(dev, { level: "metadata", valuesCompared: false }),
      ],
      keys: [
        {
          key: "API_URL",
          base: 1,
          others: [
            { version: 1, sameAsBase: true },
            { version: 1, sameAsBase: null },
          ],
        },
      ],
    });
    const { c } = panel();
    await vi.waitFor(() => expect(c.rows.value).toHaveLength(1));

    expect(c.rows.value[0]!.cells[1]!.status).toBe("unknown");
    expect(c.rows.value[0]!.different).toBe(false);
    expect(c.namesOnly.value.map((col) => col.name)).toEqual(["dev"]);
  });
});

describe("useComparePanelController — filling a Not set cell", () => {
  it("matches the base, copied on the server", async () => {
    const { c } = panel();
    await vi.waitFor(() => expect(c.rows.value).toHaveLength(4));
    const sentry = c.rows.value.find((r) => r.key === "SENTRY_DSN")!;

    c.openFill(sentry, sentry.cells[1]!);
    expect(c.fillDialog.value?.sources.map((s) => s.column.name)).toEqual([
      "prod",
    ]);
    expect(c.fillDialog.value?.choice).toBe("env_prod");
    await c.confirmFill();

    expect(compareApi.copySecret).toHaveBeenCalledWith(
      "env_dev",
      "SENTRY_DSN",
      "env_prod",
      null,
    );
    expect(c.fillDialog.value).toBeNull();
    expect(c.filled.value).toBe("SENTRY_DSN set in dev.");
    await vi.waitFor(() =>
      expect(compareApi.compareEnvironments).toHaveBeenCalledTimes(2),
    );
  });

  it("offers every environment that has the key when the base lacks it", async () => {
    const { c } = panel();
    await vi.waitFor(() => expect(c.rows.value).toHaveLength(4));
    const flags = c.rows.value.find((r) => r.key === "FEATURE_FLAGS")!;

    c.openFill(flags, flags.base);

    expect(c.fillDialog.value?.sources.map((s) => s.column.name)).toEqual([
      "staging",
      "dev",
    ]);
  });

  it("saves a value of its own", async () => {
    const { c } = panel();
    await vi.waitFor(() => expect(c.rows.value).toHaveLength(4));
    const flags = c.rows.value.find((r) => r.key === "FEATURE_FLAGS")!;
    c.openFill(flags, flags.base);
    c.fillDialog.value!.choice = "own";

    await c.confirmFill();
    expect(c.fillError.value).toBe(
      "Enter a value, or match another environment.",
    );
    expect(secretsApi.setSecret).not.toHaveBeenCalled();

    c.fillDialog.value!.value = "beta";
    await c.confirmFill();
    expect(secretsApi.setSecret).toHaveBeenCalledWith(
      "env_prod",
      "FEATURE_FLAGS",
      "beta",
    );
    expect(compareApi.copySecret).not.toHaveBeenCalled();
    expect(c.fillDialog.value).toBeNull();
  });

  it("explains a fill that lost a race", async () => {
    vi.mocked(compareApi.copySecret).mockRejectedValue(
      new ApiError(409, { SECRET_VERSION_CONFLICT: "changed" }),
    );
    const { c } = panel();
    await vi.waitFor(() => expect(c.rows.value).toHaveLength(4));
    const flags = c.rows.value.find((r) => r.key === "FEATURE_FLAGS")!;

    c.openFill(flags, flags.base);
    await c.confirmFill();

    expect(c.fillDialog.value).not.toBeNull();
    expect(c.fillError.value).toContain("set since this page loaded");
  });
});
