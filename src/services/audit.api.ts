import { apiRequest } from "./http.client";

export interface AuditEvent {
  id: string;
  /**
   * `"root"` for the root user, `"admin"` for other administrators,
   * `"runner"`, `"anonymous"`, `"bootstrap"`, or `"system"`.
   */
  actorType: string;
  actorId: string | null;
  actorLabel: string | null;
  /** Dot-separated action, e.g. `"secret.revealed"`. */
  action: string;
  projectId: string | null;
  environmentId: string | null;
  resourceType: string | null;
  resourceId: string | null;
  /** Client address, or null for records written outside a request. */
  ipAddress: string | null;
  metadata: Record<string, unknown>;
  createdAt: string;
  /**
   * Position in the audit hash chain and the hash of this record. Absent
   * for records written before the chain existed.
   */
  seq?: number | null;
  recordHash?: string | null;
}

export interface AuditQuery {
  cursor?: string;
  limit?: number;
  action?: string;
  projectId?: string;
  environmentId?: string;
  actor?: string;
}

export interface AuditPage {
  items: AuditEvent[];
  nextCursor: string | null;
}

const BASE = "/api/v1/audit-events";

export async function listAuditEvents(
  query: AuditQuery,
  signal?: AbortSignal,
): Promise<AuditPage> {
  const params = new URLSearchParams();
  if (query.cursor) params.set("cursor", query.cursor);
  if (query.limit) params.set("limit", String(query.limit));
  if (query.action) params.set("action", query.action);
  if (query.projectId) params.set("projectId", query.projectId);
  if (query.environmentId) params.set("environmentId", query.environmentId);
  if (query.actor) params.set("actor", query.actor);
  const search = params.toString();
  const { data } = await apiRequest<AuditPage>(
    search ? `${BASE}?${search}` : BASE,
    { signal },
  );
  return data;
}

/** The first record whose stored hashes don't match. */
export interface ChainBreak {
  seq: number;
  id: string;
  reason: string;
}

/** The outcome of recomputing every audit record's hash in order. */
export interface ChainVerifyReport {
  /** Every record matched and none sits outside the chain. */
  ok: boolean;
  /** Records checked before the first break. */
  records: number;
  headSeq: number | null;
  headHash: string | null;
  /** Records from before the chain, which it can't vouch for. */
  legacy: number;
  /** Records written outside the chain. Should stay at zero. */
  unsequenced: number;
  prunedSeq: number | null;
  broken: ChainBreak | null;
}

/** Needs `audit.verify`. Reads no secret values. */
export async function verifyAuditChain(
  signal?: AbortSignal,
): Promise<ChainVerifyReport> {
  const { data } = await apiRequest<ChainVerifyReport>(`${BASE}/verify`, {
    signal,
  });
  return data;
}
