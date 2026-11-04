import { describe, expect, it, vi } from "vitest";
import { useQuery, useQueryCache } from "@pinia/colada";
import { mountController } from "~/tests/mount-controller";
import { keys } from "./keys";
import { useInvalidate } from "./invalidate";
import { readQuery } from "./helpers";

function mountReads() {
  const secretsA = vi.fn().mockResolvedValue(["A"]);
  const secretsB = vi.fn().mockResolvedValue(["B"]);
  const environments = vi.fn().mockResolvedValue(["dev"]);
  const { controller } = mountController(() => ({
    a: useQuery({ key: keys.secrets.of("env_a"), query: secretsA }),
    b: useQuery({ key: keys.secrets.of("env_b"), query: secretsB }),
    list: useQuery({ key: keys.environments.list(null), query: environments }),
    invalidate: useInvalidate(),
  }));
  return { controller, secretsA, secretsB, environments };
}

describe("useInvalidate", () => {
  it("refreshes the reads a change affects and leaves the rest alone", async () => {
    const { controller, secretsA, secretsB, environments } = mountReads();
    await vi.waitFor(() => expect(controller.list.data.value).toEqual(["dev"]));

    await controller.invalidate.secretsChanged("env_a");

    expect(secretsA).toHaveBeenCalledTimes(2);
    expect(environments).toHaveBeenCalledTimes(2);
    expect(secretsB).toHaveBeenCalledTimes(1);
  });

  it("refreshes project lists off screen too after the tree changes", async () => {
    const offScreen = vi.fn().mockResolvedValue(["dev", "prod"]);
    const { controller } = mountController(() => ({
      cache: useQueryCache(),
      invalidate: useInvalidate(),
    }));
    // Read once by a screen that has closed since.
    await readQuery(controller.cache, {
      key: keys.environments.list(null),
      query: offScreen,
    });
    offScreen.mockResolvedValue(["dev", "production"]);

    await controller.invalidate.projectTreeChanged();

    expect(offScreen).toHaveBeenCalledTimes(2);
    expect(controller.cache.getQueryData(keys.environments.list(null))).toEqual(
      ["dev", "production"],
    );
  });

  it("empties affected reads no screen shows, so they are never shown out of date", async () => {
    const { controller } = mountController(() => ({
      cache: useQueryCache(),
      invalidate: useInvalidate(),
    }));
    // Read by a screen that has closed since.
    controller.cache.setQueryData(keys.secrets.of("env_a"), ["A"]);
    controller.cache.setQueryData(keys.secrets.of("env_b"), ["B"]);

    await controller.invalidate.secretsChanged("env_a");

    expect(controller.cache.getQueryData(keys.secrets.of("env_a"))).toBe(
      undefined,
    );
    expect(controller.cache.getQueryData(keys.secrets.of("env_b"))).toEqual([
      "B",
    ]);
  });

  it("resolves even when a refetch fails", async () => {
    const { controller, secretsA } = mountReads();
    await vi.waitFor(() => expect(controller.a.data.value).toEqual(["A"]));
    secretsA.mockRejectedValueOnce(new Error("down"));

    await expect(
      controller.invalidate.secretsChanged("env_a"),
    ).resolves.toBeUndefined();
    expect(controller.a.data.value).toEqual(["A"]);
  });
});
