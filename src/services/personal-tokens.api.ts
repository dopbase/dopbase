import { apiRequest } from "./http.client";
import type { Level, ScopeType } from "./access.api";

/** A personal token's metadata. The value is shown once, at creation. */
export interface PersonalToken {
  id: string;
  userId: string;
  name: string;
  scopeType: ScopeType;
  scopeId: string;
  /** `project` or `project/environment`. Empty for all projects. */
  scopeLabel: string;
  level: Level;
  createdBy: string | null;
  createdAt: string;
  expiresAt: string | null;
  lastUsedAt: string | null;
  revokedAt: string | null;
}

export interface CreatedPersonalToken {
  token: PersonalToken;
  plaintextToken: string;
}

export interface CreatePersonalTokenRequest {
  name: string;
  /** Audit tokens take none. */
  level?: Level;
  scopeType?: ScopeType;
  /** Project id or name, or environment id. */
  scope?: string;
  /** Such as `12h` or `30d`. Personal tokens always expire. */
  expiresIn?: string;
}

/** `userId` is `me` for your own tokens. */
function base(userId: string): string {
  return `/api/v1/users/${encodeURIComponent(userId)}/tokens`;
}

export async function listPersonalTokens(
  userId = "me",
): Promise<PersonalToken[]> {
  const { data } = await apiRequest<PersonalToken[]>(base(userId));
  return data;
}

export async function createPersonalToken(
  userId: string,
  request: CreatePersonalTokenRequest,
): Promise<CreatedPersonalToken> {
  const { data } = await apiRequest<CreatedPersonalToken>(base(userId), {
    method: "POST",
    body: request,
  });
  return data;
}

export async function revokePersonalToken(
  userId: string,
  tokenId: string,
): Promise<PersonalToken> {
  const { data } = await apiRequest<PersonalToken>(
    `${base(userId)}/${encodeURIComponent(tokenId)}/revoke`,
    { method: "POST" },
  );
  return data;
}
