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

export interface ExportSecretsResponse {
  entries: SecretEntry[];
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
 * validate the whole batch and preview the effect without mutating anything.
 */
export async function importSecrets(
  environmentId: string,
  request: ImportSecretsRequest,
): Promise<ImportSecretsResponse> {
  const { data } = await apiRequest<ImportSecretsResponse>(
    `${base(environmentId)}/import`,
    { method: "POST", body: request },
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
