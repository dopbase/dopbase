import { apiRequest } from "./http.client";

export interface AuditEvent {
  id: string;
  /** `"admin"`, `"anonymous"`, or `"runner"`. */
  actorType: string;
  actorId: string | null;
  actorLabel: string | null;
  /** Dot-separated action, e.g. `"secret.revealed"`. */
  action: string;
  projectId: string | null;
  environmentId: string | null;
  resourceType: string | null;
  resourceId: string | null;
  metadata: Record<string, unknown>;
  createdAt: string;
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

export async function listAuditEvents(query: AuditQuery): Promise<AuditPage> {
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
  );
  return data;
}
