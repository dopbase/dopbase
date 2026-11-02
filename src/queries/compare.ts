import { defineQueryOptions } from "@pinia/colada";
import { compareEnvironments } from "~/services/compare.api";
import { keys } from "./keys";

/**
 * One environment's keys read against others: which are set, missing or
 * differ. Values are never part of it; a fill sends one with its own call.
 */
export const compareQuery = defineQueryOptions(
  ({ baseId, withIds }: { baseId: string; withIds: readonly string[] }) => ({
    key: keys.compare.of(baseId, withIds),
    query: ({ signal }) => compareEnvironments(baseId, withIds, signal),
  }),
);
