import type { AuditEvent } from "~/services/audit.api";
import { LEVELS, type Level } from "~/services/access.api";
import { formatLevel } from "./format";

/** Failures stand out; everything else reads as identity. */
export function actionTone(action: string): "crit" | "accent" {
  return /(failed|denied)$/.test(action) ? "crit" : "accent";
}

/** `203.0.113.24 · dopbase-cli 0.2.0`, or empty without an address. */
export function eventOrigin(event: AuditEvent): string {
  const client = event.metadata.client;
  return [event.ipAddress, typeof client === "string" ? client : null]
    .filter(Boolean)
    .join(" · ");
}

const text = (value: unknown): string | null =>
  typeof value === "string" && value !== "" ? value : null;
const count = (value: unknown): number | null =>
  typeof value === "number" ? value : null;

/**
 * What an event touched, in a few words, from its metadata:
 * `maya@dopbase.com · Write`, `staging → qa`, `DATABASE_URL v3`. Empty when
 * the record holds nothing worth naming.
 */
export function eventDetail(event: AuditEvent): string {
  const m = event.metadata;
  const principal = text(m.principal);
  if (principal) {
    const level = text(m.level);
    return level && LEVELS.includes(level as Level)
      ? `${principal} · ${formatLevel(level as Level)}`
      : principal;
  }
  const renamedFrom = text(m.oldName);
  const renamedTo = text(m.newName) ?? text(m.name);
  if (renamedFrom && renamedTo) return `${renamedFrom} → ${renamedTo}`;
  const group = text(m.group);
  const email = text(m.email);
  if (group) return email ? `${email} in ${group}` : group;
  if (email) return email;
  const key = text(m.key) ?? text(m.backup);
  if (key) {
    const version = count(m.version);
    return version === null ? key : `${key} v${version}`;
  }
  const name = text(m.name) ?? text(m.issuer) ?? text(m.hostname);
  if (name) return name;
  const entries = count(m.count) ?? count(m.entryCount) ?? count(m.secretCount);
  if (entries !== null) return `${entries} ${entries === 1 ? "key" : "keys"}`;
  const kind = text(m.kind);
  if (kind) return kind === "cli" ? "CLI" : kind;
  return text(m.reason) ?? "";
}
