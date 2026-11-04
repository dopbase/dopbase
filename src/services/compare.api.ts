import { apiRequest } from "./http.client";
import type { Level } from "./access.api";
import type { SecretMetadata } from "./secrets.api";

/** How many environments one comparison reads against its base. */
export const MAX_COMPARE_ENVIRONMENTS = 3;

export interface CompareEnvironment {
  id: string;
  name: string;
  protected: boolean;
  level: Level;
  /** False when only which keys exist is known here (no Read on both sides). */
  valuesCompared: boolean;
}

export interface CompareCell {
  /** The current version, or null when the key isn't set there. */
  version: number | null;
  /** Whether the value matches the base; null when it wasn't compared. */
  sameAsBase: boolean | null;
}

export interface CompareKey {
  key: string;
  /** The base's current version, or null when the base lacks the key. */
  base: number | null;
  /** One cell per compared environment, in request order. */
  others: CompareCell[];
}

export interface CompareResult {
  base: CompareEnvironment;
  environments: CompareEnvironment[];
  keys: CompareKey[];
}

/**
 * Lines up the base environment's keys with up to three others in its
 * project. The server never returns values, only whether each matches.
 */
export async function compareEnvironments(
  baseId: string,
  withIds: readonly string[],
  signal?: AbortSignal,
): Promise<CompareResult> {
  const query = new URLSearchParams({ with: withIds.join(",") });
  const { data } = await apiRequest<CompareResult>(
    `/api/v1/environments/${encodeURIComponent(baseId)}/compare?${query}`,
    { signal },
  );
  return data;
}

/**
 * Copies another environment's current value of `key` into `targetId` as a
 * new version. The value is copied on the server and never sent here.
 * `expectedVersion` is the target's version you saw, or null if it had none.
 */
export async function copySecret(
  targetId: string,
  key: string,
  fromId: string,
  expectedVersion: number | null,
): Promise<SecretMetadata> {
  const { data } = await apiRequest<SecretMetadata>(
    `/api/v1/environments/${encodeURIComponent(targetId)}/secrets/${encodeURIComponent(key)}/copy`,
    {
      method: "POST",
      body: {
        from: fromId,
        ...(expectedVersion === null ? {} : { expectedVersion }),
      },
    },
  );
  return data;
}
