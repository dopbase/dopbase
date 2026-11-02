import { apiRequest } from "./http.client";

/** Safe, read-only instance status. Never exposes paths or key material. */
export interface InstanceStatus {
  version: string;
  publicUrl: string;
  initializationState: string;
  databaseHealth: string;
  keyAvailability: string;
  /** Configuration changes still require a restart in v0.1.0. */
  configurationReload: string;
}
export type CheckStatus = "crit" | "warn" | "tip" | "ok";

/** One health check result. Errors in a check come back as a warning. */
export interface StatusCheck {
  /** Stable identifier, such as `backup.age`. */
  id: string;
  status: CheckStatus;
  title: string;
  detail: string;
  /** Short reading shown beside a passing check, such as `3 ms`. */
  value: string | null;
  /** Admin UI path or documentation URL that fixes the problem. */
  fixPath: string | null;
  lastRunAt: string;
}

/** Check counts by result. */
export interface StatusSummary {
  critical: number;
  warnings: number;
  tips: number;
  passed: number;
  checkedAt: string;
}

export interface HealthReport {
  database: {
    status: CheckStatus;
    pingMs: number | null;
    sizeBytes: number | null;
  };
  masterKey: {
    status: CheckStatus;
    /** How the key file compares with the key the server loaded. */
    file: "matches" | "changed" | "missing" | "unreadable";
  };
  /** Null where free space can't be read. */
  disk: { status: CheckStatus; freeBytes: number; totalBytes: number } | null;
  auditLog: {
    status: CheckStatus;
    events: number | null;
    intact: boolean | null;
    checkedAt: string | null;
  };
}

/** The last 24 hours in hourly buckets, oldest first. */
export interface StatusActivity {
  hours: string[];
  secretReads: {
    total: number;
    hourly: number[];
    runnerTokens: number;
    people: number;
    aiAgents: number;
  };
  failedSignIns: {
    total: number;
    hourly: number[];
    /** Null without `audit.read`. */
    accounts: string[] | null;
    addresses: string[] | null;
  };
  rateLimited: { total: number; hourly: number[] };
  activeSessions: { browser: number; cli: number };
}

export interface LatestRelease {
  version: string;
  updateAvailable: boolean;
  url: string;
  checkedAt: string;
}

/**
 * Status, counts and health checks. Counts are null when their query
 * failed; checks, health and activity are null without `instance.view`.
 */
export interface StatusResponse {
  version: string;
  uptimeSeconds: number;
  initializationState: string;
  platform: { os: string; arch: string };
  projects: number | null;
  environments: number | null;
  secrets: number | null;
  humanUsers: number | null;
  aiAgents: number | null;
  activeRunnerTokens: number | null;
  activeAgentTokens: number | null;
  backups: number;
  summary: StatusSummary | null;
  checks: StatusCheck[] | null;
  health: HealthReport | null;
  activity: StatusActivity | null;
  latestRelease: LatestRelease | null;
  observedAt: string;
}

export interface FactoryResetPreview {
  users: number;
  projects: number;
  environments: number;
  secrets: number;
  runnerTokens: number;
  backups: number;
  aiAgents: number;
  agentTokens: number;
}

const BASE = "/api/v1/instance";

export async function fetchInstanceStatus(): Promise<InstanceStatus> {
  const { data } = await apiRequest<InstanceStatus>(BASE);
  return data;
}
export async function fetchStatus(
  signal?: AbortSignal,
): Promise<StatusResponse> {
  const { data } = await apiRequest<StatusResponse>("/api/v1/status", {
    signal,
  });
  return data;
}

/** Run every check now. Refused with 429 within 10 seconds of the last run. */
export async function runStatusChecks(): Promise<StatusResponse> {
  const { data } = await apiRequest<StatusResponse>(
    "/api/v1/status/checks/run",
    { method: "POST" },
  );
  return data;
}

export async function fetchFactoryResetPreview(): Promise<FactoryResetPreview> {
  const { data } = await apiRequest<FactoryResetPreview>(
    `${BASE}/factory-reset`,
  );
  return data;
}

export async function factoryReset(
  currentPassword: string,
  confirmation: string,
  acknowledged: boolean,
): Promise<void> {
  await apiRequest(`${BASE}/factory-reset`, {
    method: "POST",
    body: { currentPassword, confirmation, acknowledged },
  });
}
