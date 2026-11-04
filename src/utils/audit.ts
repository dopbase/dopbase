import type { AuditEvent } from "~/services/audit.api";

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
