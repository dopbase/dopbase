import { apiRequest } from "./http.client";
import type { SecretEntry } from "./projects.api";

/** Metadata only — secret values are never part of a listing. */
export interface SecretMetadata {
  key: string;
  version: number;
  createdAt: string;
  updatedAt: string;
}

export interface RevealedSecret {
  key: string;
  value: string;
  version: number;
}

export type ImportMode = "merge" | "replace";

export interface ImportSecretsRequest {
  mode: ImportMode;
  dryRun: boolean;
  entries: SecretEntry[];
  /**
   * `.env` editor layout (comments, ordering, empty `KEY=` slots — never
   * values). Persisted alongside a non-dry-run import.
   */
  envLayout?: string;
  /** Revision returned by the dry run being confirmed. */
  expectedRevision?: string;
}

export interface ImportSecretsResponse {
  addedKeys: string[];
  updatedKeys: string[];
  unchangedKeys: string[];
  deletedKeys: string[];
  dryRun: boolean;
  revision: string;
}

export interface CopySecretsResponse {
  /** Keys created in the target, A to Z. */
  copiedKeys: string[];
  /** Keys left alone because the target already had them. */
  skippedKeys: string[];
}

export interface ExportSecretsResponse {
  entries: SecretEntry[];
  revision: string;
  envLayout: string | null;
}

const base = (environmentId: string): string =>
  `/api/v1/environments/${encodeURIComponent(environmentId)}/secrets`;

export async function listSecrets(
  environmentId: string,
  signal?: AbortSignal,
): Promise<SecretMetadata[]> {
  const { data } = await apiRequest<SecretMetadata[]>(base(environmentId), {
    signal,
  });
  return data;
}

export async function getSecret(
  environmentId: string,
  key: string,
): Promise<SecretMetadata> {
  const { data } = await apiRequest<SecretMetadata>(
    `${base(environmentId)}/${encodeURIComponent(key)}`,
  );
  return data;
}

export async function setSecret(
  environmentId: string,
  key: string,
  value: string,
): Promise<SecretMetadata> {
  const { data } = await apiRequest<SecretMetadata>(
    `${base(environmentId)}/${encodeURIComponent(key)}`,
    { method: "PUT", body: { value } },
  );
  return data;
}

export async function deleteSecret(
  environmentId: string,
  key: string,
): Promise<void> {
  await apiRequest(`${base(environmentId)}/${encodeURIComponent(key)}`, {
    method: "DELETE",
  });
}

/** Reveals one plaintext value. Requires recent password authentication. */
export async function revealSecret(
  environmentId: string,
  key: string,
): Promise<RevealedSecret> {
  const { data } = await apiRequest<RevealedSecret>(
    `${base(environmentId)}/${encodeURIComponent(key)}/reveal`,
    { method: "POST" },
  );
  return data;
}

/**
 * The stored `.env` editor layout (comments, ordering, empty `KEY=` slots).
 * Contains no secret values, so no recent password authentication is needed.
 */
export interface EnvLayoutResponse {
  layout: string | null;
}

export async function getEnvLayout(
  environmentId: string,
): Promise<EnvLayoutResponse> {
  const { data } = await apiRequest<EnvLayoutResponse>(
    `${base(environmentId)}/layout`,
  );
  return data;
}

/**
 * Imports entries in `merge` or `replace` mode. Run with `dryRun: true` to
 * validate the batch without changing secrets. Both modes require recent
 * browser password confirmation; successful previews create an audit event.
 */
export async function importSecrets(
  environmentId: string,
  request: ImportSecretsRequest,
  signal?: AbortSignal,
): Promise<ImportSecretsResponse> {
  const { data } = await apiRequest<ImportSecretsResponse>(
    `${base(environmentId)}/import`,
    { method: "POST", body: request, signal },
  );
  return data;
}

/** Exports every secret plaintext. Requires recent password authentication. */
export async function exportSecrets(
  environmentId: string,
): Promise<ExportSecretsResponse> {
  const { data } = await apiRequest<ExportSecretsResponse>(
    `${base(environmentId)}/export`,
    { method: "POST" },
  );
  return data;
}

export type SecretChangeType =
  "created" | "updated" | "imported" | "rolled_back" | "legacy";

/** One stored version of a secret. Never contains the value. */
export interface SecretVersion {
  version: number;
  changeType: SecretChangeType;
  /** The version a rollback restored. */
  sourceVersion: number | null;
  actorType: string;
  actorLabel: string | null;
  createdAt: string;
  current: boolean;
  /** False once retention has removed the value. */
  restorable: boolean;
}

export interface SecretVersionPage {
  /** Newest first. */
  items: SecretVersion[];
  /** Pass as `before` for the next page; null on the last page. */
  nextBefore: number | null;
  /** How many of the newest versions keep their value. */
  retainedVersions: number;
}

export interface RollbackSecretResponse {
  key: string;
  version: number;
  sourceVersion: number;
  previousVersion: number;
  createdAt: string;
  updatedAt: string;
}

export async function listSecretVersions(
  environmentId: string,
  key: string,
  options: { before?: number; limit?: number; signal?: AbortSignal } = {},
): Promise<SecretVersionPage> {
  const params = new URLSearchParams();
  if (options.limit !== undefined) params.set("limit", String(options.limit));
  if (options.before !== undefined)
    params.set("before", String(options.before));
  const query = params.size > 0 ? `?${params.toString()}` : "";
  const { data } = await apiRequest<SecretVersionPage>(
    `${base(environmentId)}/${encodeURIComponent(key)}/versions${query}`,
    { signal: options.signal },
  );
  return data;
}

/** Reveals one earlier value. Requires recent password authentication. */
export async function revealSecretVersion(
  environmentId: string,
  key: string,
  version: number,
): Promise<RevealedSecret> {
  const { data } = await apiRequest<RevealedSecret>(
    `${base(environmentId)}/${encodeURIComponent(key)}/versions/${version}/reveal`,
    { method: "POST" },
  );
  return data;
}

/**
 * Saves the value of `version` as a new current version. The server refuses
 * with `SECRET_VERSION_CONFLICT` when the current version is no longer
 * `expectedVersion`. Requires recent password authentication.
 */
export async function rollbackSecret(
  environmentId: string,
  key: string,
  version: number,
  expectedVersion: number,
): Promise<RollbackSecretResponse> {
  const { data } = await apiRequest<RollbackSecretResponse>(
    `${base(environmentId)}/${encodeURIComponent(key)}/rollback`,
    { method: "POST", body: { version, expectedVersion } },
  );
  return data;
}

/**
 * Creates `keys` in `targetId` from another environment in the same
 * project. With `values`, each key gets the source's current value, copied
 * on the server and never sent here; without it each key starts empty.
 * Keys the target already has are skipped.
 */
export async function copySecrets(
  targetId: string,
  from: string,
  keys: string[],
  values: boolean,
): Promise<CopySecretsResponse> {
  const { data } = await apiRequest<CopySecretsResponse>(
    `/api/v1/environments/${encodeURIComponent(targetId)}/copy-secrets`,
    { method: "POST", body: { from, keys, values } },
  );
  return data;
}
