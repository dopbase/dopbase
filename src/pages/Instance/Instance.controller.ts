import { computed, onMounted, onUnmounted, ref, watch } from "vue";
import { useRouter } from "vue-router";
import { useQuery, useQueryCache } from "@pinia/colada";
import { useAuthStore } from "~/stores/auth.store";
import { useReauthentication } from "~/composable/reauthentication";
import { factoryReset, runStatusChecks } from "~/services/instance.api";
import type {
  CheckStatus,
  StatusCheck,
  StatusResponse,
} from "~/services/instance.api";
import type { Capability } from "~/services/access.api";
import { downloadMasterKeyBlob } from "~/services/backups.api";
import { ApiError } from "~/services/http.client";
import { errorMessage, isAbortError } from "~/services/api-errors";
import { loadFailure } from "~/queries/helpers";
import { statusQuery } from "~/queries/instance";
import { saveBlob } from "~/utils/download";
import { copyText } from "~/utils/clipboard";
import {
  formatBytes,
  formatDateTime,
  formatDuration,
  formatList,
  formatRelativeTime,
} from "~/utils/format";

/** What the user types to confirm a factory reset. */
export const RESET_PHRASE = "FACTORY RESET";

/** How often the page asks for fresh results while it is open. */
export const POLL_INTERVAL_MS = 30_000;

/** Where the master key docs explain putting the right key file back. */
export const MASTER_KEY_DOCS =
  "https://docs.dopbase.com/self-hosting/encryption-keys";

export type Tone = "ok" | "warn" | "crit" | "neutral" | "accent";

export interface FixAction {
  label: string;
  icon: "archive" | "download" | "key" | "external" | "audit";
  /** An Admin UI path, a documentation URL, or the root-only key download. */
  kind: "route" | "external" | "master-key";
  to: string;
}

export interface HealthCard {
  id: "database" | "masterKey" | "disk" | "auditLog";
  label: string;
  value: string;
  /** Left out where the reading carries no status, as on Disk usage. */
  badge?: { tone: Tone; text: string };
  detail: string;
}

const SEVERITY_LABELS: Record<Exclude<CheckStatus, "ok">, string> = {
  crit: "Critical",
  warn: "Warning",
  tip: "Tip",
};

const SEVERITY_TONES: Record<CheckStatus, Tone> = {
  crit: "crit",
  warn: "warn",
  tip: "neutral",
  ok: "ok",
};

function plural(count: number, one: string, many: string): string {
  return `${count.toLocaleString("en-US")} ${count === 1 ? one : many}`;
}

/** The capability an Admin UI path needs, if any. */
function capabilityFor(path: string): Capability | null {
  if (path.startsWith("/backups")) return "instance.backup";
  if (path.startsWith("/audit")) return "audit.read";
  if (path.startsWith("/users")) return "users.manage";
  return null;
}

/**
 * Status screen controller: the server's health checks, the readings behind
 * them, the last day of activity, and what it stores. Root also gets the
 * master key download and the factory reset.
 *
 * The page polls every 30 seconds. A failed poll keeps the last results on
 * screen and says when they are from.
 */
export function useInstanceController() {
  const router = useRouter();
  const auth = useAuthStore();
  const { runWithReauth } = useReauthentication();
  const cache = useQueryCache();
  const statusList = useQuery(statusQuery);
  const status = computed<StatusResponse | null>(
    () => statusList.data.value ?? null,
  );
  const loading = statusList.isLoading;
  const loadError = loadFailure(
    statusList,
    "Could not load the instance status.",
  );
  /** Set when a poll fails while older results are still on screen. */
  const staleSince = computed(() =>
    statusList.error.value && status.value ? status.value.observedAt : null,
  );
  const running = ref(false);
  const runError = ref<string | null>(null);
  const showPassed = ref(false);
  const diagnosticsCopied = ref(false);
  const resetOpen = ref(false);

  const isRoot = computed(() => auth.isRoot);

  const downloadingMasterKey = ref(false);
  const masterKeyMessage = ref<{ text: string; tone: "ok" | "crit" } | null>(
    null,
  );

  const resetPassword = ref("");
  const resetConfirmation = ref("");
  const resetAcknowledged = ref(false);
  const resetLoading = ref(false);
  const resetError = ref<string | null>(null);
  const resetComplete = ref(false);

  /** The reset button stays off until every safeguard is filled in. */
  const canReset = computed(
    () =>
      isRoot.value &&
      resetAcknowledged.value &&
      resetConfirmation.value === RESET_PHRASE &&
      resetPassword.value !== "",
  );

  const bootTime = computed(() => {
    if (!status.value) return null;
    const observedMs = new Date(status.value.observedAt).getTime();
    const baseTime = Number.isNaN(observedMs) ? Date.now() : observedMs;
    return new Date(baseTime - status.value.uptimeSeconds * 1000).toISOString();
  });

  const formattedUptime = computed(() => {
    if (!status.value) return "";
    return formatDuration(status.value.uptimeSeconds);
  });

  const formattedObservedAt = computed(() => {
    if (!status.value) return "";
    return formatDateTime(status.value.observedAt);
  });

  const relativeObservedAt = computed(() => {
    if (!status.value) return "";
    return formatRelativeTime(status.value.observedAt);
  });

  const relativeBootTime = computed(() => {
    if (!bootTime.value) return "";
    return formatRelativeTime(bootTime.value);
  });

  const checks = computed<StatusCheck[]>(() => status.value?.checks ?? []);
  /** Failing checks, most severe first, as the server orders them. */
  const attention = computed(() =>
    checks.value.filter((check) => check.status !== "ok"),
  );
  const passed = computed(() =>
    checks.value.filter((check) => check.status === "ok"),
  );
  const hasChecks = computed(() => status.value?.checks != null);

  /** "Dopbase 0.2.0 · up 6d 4h" in the banner. */
  const serverLine = computed(() =>
    status.value
      ? `Dopbase ${status.value.version} · up ${formattedUptime.value}`
      : "",
  );

  const releaseBadge = computed<{ tone: Tone; text: string } | null>(() => {
    const release = status.value?.latestRelease;
    if (!release) return null;
    return release.updateAvailable
      ? { tone: "accent", text: `${release.version.replace(/^v/, "")} is out` }
      : { tone: "ok", text: "latest version" };
  });

  const banner = computed<{
    tone: Tone;
    title: string;
    detail: string;
    keyProblem: boolean;
  } | null>(() => {
    const current = status.value;
    if (!current?.summary || !current.checks) return null;
    const checked = `Checked ${formatRelativeTime(current.summary.checkedAt)}.`;
    const keyFile = current.health?.masterKey.file;
    if (keyFile && keyFile !== "matches") {
      const title = {
        changed: "The master key file changed on disk",
        missing: "The master key file is missing",
        unreadable: "The master key file can't be read",
      }[keyFile];
      return {
        tone: "crit",
        title,
        detail:
          "The server still runs on the key it loaded at startup, but it won't start again after a restart. Put the right key file back first.",
        keyProblem: true,
      };
    }
    const { critical, warnings, tips, passed: ok } = current.summary;
    const failing = critical + warnings + tips;
    if (failing === 0) {
      return {
        tone: "ok",
        title: "Everything looks good",
        detail: `All ${plural(ok, "check", "checks")} passed. ${checked}`,
        keyProblem: false,
      };
    }
    const parts = [
      critical ? `${critical} critical` : null,
      warnings ? plural(warnings, "warning", "warnings") : null,
      tips ? plural(tips, "tip", "tips") : null,
    ].filter((part): part is string => part !== null);
    const others = ok
      ? ` ${plural(ok, "other check", "other checks")} passed.`
      : "";
    return {
      tone: critical ? "crit" : "warn",
      title: `${failing} ${failing === 1 ? "thing needs" : "things need"} attention`,
      detail: `${formatList(parts)}.${others} ${checked}`,
      keyProblem: false,
    };
  });

  const healthCards = computed<HealthCard[]>(() => {
    const health = status.value?.health;
    if (!health) return [];
    const { database, masterKey, disk, auditLog } = health;
    const ping =
      database.pingMs == null
        ? null
        : database.pingMs < 5
          ? "under 5 ms"
          : `${Math.round(database.pingMs)} ms`;
    const databaseCard: HealthCard = {
      id: "database",
      label: "Database",
      value: { ok: "Healthy", warn: "Slow", tip: "Healthy", crit: "Down" }[
        database.status
      ],
      badge: {
        tone: SEVERITY_TONES[database.status],
        text:
          database.status === "crit"
            ? "down"
            : database.status === "warn"
              ? "slow"
              : "ok",
      },
      detail: [
        ping ? `Latency ${ping}` : "Not answering",
        database.sizeBytes == null ? null : formatBytes(database.sizeBytes),
      ]
        .filter(Boolean)
        .join(" · "),
    };
    const keyCard: HealthCard =
      masterKey.file === "matches"
        ? {
            id: "masterKey",
            label: "Master key",
            value: "Verified",
            badge: { tone: "ok", text: "ok" },
            detail: "Matches this database",
          }
        : {
            id: "masterKey",
            label: "Master key",
            value: {
              changed: "File changed",
              missing: "File missing",
              unreadable: "File unreadable",
            }[masterKey.file],
            badge: { tone: "crit", text: "at risk" },
            detail: "Key in use still works",
          };
    const diskCard: HealthCard = disk
      ? {
          id: "disk",
          label: "Disk usage",
          value: formatBytes(disk.usedBytes),
          detail: "Storage used",
        }
      : {
          id: "disk",
          label: "Disk usage",
          value: "Unknown",
          detail: "Couldn't read the data directory",
        };
    const auditCard: HealthCard = {
      id: "auditLog",
      label: "Audit log",
      value:
        auditLog.intact == null
          ? "Unknown"
          : auditLog.intact
            ? "Intact"
            : "Broken",
      badge: {
        tone: SEVERITY_TONES[auditLog.status],
        text:
          auditLog.intact === false
            ? "broken"
            : auditLog.intact
              ? "ok"
              : "unknown",
      },
      detail: auditLog.checkedAt
        ? `Chain checked ${formatRelativeTime(auditLog.checkedAt)}`
        : "Not checked yet",
    };
    return [databaseCard, keyCard, diskCard, auditCard];
  });

  const activityTiles = computed(() => {
    const activity = status.value?.activity;
    if (!activity) return [];
    const { secretReads, failedSignIns, rateLimited, activeSessions } =
      activity;
    const readers = [
      secretReads.runnerTokens
        ? plural(secretReads.runnerTokens, "runner token", "runner tokens")
        : null,
      secretReads.people
        ? plural(secretReads.people, "person", "people")
        : null,
      secretReads.aiAgents
        ? plural(secretReads.aiAgents, "AI agent", "AI agents")
        : null,
    ].filter((part): part is string => part !== null);
    let signInDetail = "None in the last day";
    if (failedSignIns.total > 0) {
      const accounts = failedSignIns.accounts ?? [];
      const addresses = failedSignIns.addresses ?? [];
      if (accounts.length === 0) {
        signInDetail = "In the last day";
      } else {
        signInDetail = `${accounts.length === 1 ? "All for" : "For"} ${formatList(accounts)}`;
        if (addresses.length) signInDetail += `, from ${formatList(addresses)}`;
      }
    }
    return [
      {
        id: "secretReads",
        label: "Secret reads",
        value: secretReads.total.toLocaleString("en-US"),
        tone: "neutral" as Tone,
        hourly: secretReads.hourly,
        detail: readers.length
          ? `By ${formatList(readers)}`
          : "None in the last day",
      },
      {
        id: "failedSignIns",
        label: "Failed sign-ins",
        value: failedSignIns.total.toLocaleString("en-US"),
        tone: (failedSignIns.total > 0 ? "warn" : "neutral") as Tone,
        hourly: null,
        detail: signInDetail,
      },
      {
        id: "rateLimited",
        label: "Rate limited",
        value: rateLimited.total.toLocaleString("en-US"),
        tone: (rateLimited.total > 0 ? "warn" : "neutral") as Tone,
        hourly: null,
        detail:
          rateLimited.total > 0
            ? "Sign-in attempts refused by the limits"
            : "No client was slowed down",
      },
      {
        id: "activeSessions",
        label: "Active sessions",
        value: (activeSessions.browser + activeSessions.cli).toLocaleString(
          "en-US",
        ),
        tone: "neutral" as Tone,
        hourly: null,
        detail: `${plural(activeSessions.browser, "browser session", "browser sessions")}, ${plural(activeSessions.cli, "CLI session", "CLI sessions")}`,
      },
    ];
  });

  /**
   * The "What's stored" grid, in the order the status API counts them.
   * Accounts and tokens carry a hint, since "AI agents" and "AI agent
   * tokens" read alike. A count the server couldn't read shows as a dash.
   */
  const counts = computed<
    { label: string; value: number | string; hint?: string }[]
  >(() => {
    const current = status.value;
    if (!current) return [];
    const shown = (value: number | null) => value ?? "—";
    return [
      { label: "Projects", value: shown(current.projects) },
      { label: "Environments", value: shown(current.environments) },
      { label: "Secrets", value: shown(current.secrets) },
      {
        label: "People",
        value: shown(current.humanUsers),
        hint: "Accounts that sign in to this instance, root included.",
      },
      {
        label: "AI agents",
        value: shown(current.aiAgents),
        hint: "Accounts for bots and CI. They never sign in and use a token to call the API.",
      },
      {
        label: "Runner tokens",
        value: shown(current.activeRunnerTokens),
        hint: "Live tokens that let a deployed app read one environment's values. You create them on an environment's Tokens tab.",
      },
      {
        label: "AI agent tokens",
        value: shown(current.activeAgentTokens),
        hint: "Live tokens held by AI agents. An agent has at most one, so this is lower than AI agents when some agents have no live token.",
      },
      { label: "Backups", value: current.backups },
    ];
  });

  /**
   * The button that fixes a failing check, or null when there is none or
   * the viewer couldn't use it.
   */
  function fixAction(check: StatusCheck): FixAction | null {
    if (check.id === "master_key.file") {
      return isRoot.value
        ? {
            label: "Download key in use",
            icon: "download",
            kind: "master-key",
            to: "",
          }
        : null;
    }
    const path = check.fixPath;
    if (!path) return null;
    if (/^https?:\/\//.test(path)) {
      return {
        label:
          check.id === "version.latest" ? "Release notes" : "How to set it",
        icon: "external",
        kind: "external",
        to: path,
      };
    }
    const needs = capabilityFor(path);
    if (needs && !auth.can(needs)) return null;
    const labels: Record<string, Pick<FixAction, "label" | "icon">> = {
      "backup.age": { label: "Create backup", icon: "archive" },
      "backup.offsite": { label: "Download latest", icon: "download" },
      "tokens.expiring": { label: "Review tokens", icon: "key" },
      "audit.chain": { label: "Open audit log", icon: "audit" },
    };
    return {
      ...(labels[check.id] ?? { label: "Open", icon: "external" }),
      kind: "route",
      to: path,
    };
  }

  async function runFix(check: StatusCheck): Promise<void> {
    const action = fixAction(check);
    if (!action) return;
    if (action.kind === "master-key") {
      await downloadMasterKey();
    } else if (action.kind === "external") {
      window.open(action.to, "_blank", "noopener");
    } else {
      await router.push(action.to);
    }
  }

  function severityLabel(status: CheckStatus): string {
    return status === "ok" ? "Passed" : SEVERITY_LABELS[status];
  }

  function severityTone(status: CheckStatus): Tone {
    return SEVERITY_TONES[status];
  }

  /** Asks for fresh results now. A failure keeps the last ones on screen. */
  async function load(): Promise<void> {
    await statusList.refetch();
  }

  // A newer reading replaces an older Run checks error.
  watch(statusList.data, () => {
    runError.value = null;
  });

  /**
   * Runs every check now. A poll already out is cancelled first, and again
   * once the run answers, so its older results can never land over the
   * fresh ones; a cancelled request never writes to the cache.
   */
  async function runChecks(): Promise<void> {
    running.value = true;
    runError.value = null;
    cache.cancelQueries({ key: statusQuery.key, exact: true });
    try {
      const fresh = await runStatusChecks();
      cache.cancelQueries({ key: statusQuery.key, exact: true });
      cache.setQueryData(statusQuery.key, fresh);
    } catch (cause: unknown) {
      runError.value =
        cause instanceof ApiError && cause.status === 429
          ? "Checks just ran. Wait a few seconds and try again."
          : errorMessage(cause, "Could not run the checks.");
    } finally {
      running.value = false;
    }
  }

  /**
   * A plain-text report to paste into an issue or a chat. It carries what
   * the page shows and nothing it hides from this viewer.
   */
  function diagnosticsText(): string {
    const current = status.value;
    if (!current) return "";
    const lines = [
      `Dopbase ${current.version} on ${current.platform.os}/${current.platform.arch}, up ${formattedUptime.value}`,
      `Observed ${current.observedAt}`,
    ];
    if (current.latestRelease) {
      lines.push(`Latest release ${current.latestRelease.version}`);
    }
    if (current.summary) {
      const { critical, warnings, tips, passed: ok } = current.summary;
      lines.push(
        `Checks: ${critical} critical, ${warnings} warnings, ${tips} tips, ${ok} passed`,
        "",
      );
    }
    for (const check of checks.value) {
      const value = check.value ? ` (${check.value})` : "";
      lines.push(`[${check.status}] ${check.title}${value}: ${check.detail}`);
    }
    const activity = current.activity;
    if (activity) {
      lines.push(
        "",
        `Last 24 hours: ${activity.secretReads.total} secret reads, ${activity.failedSignIns.total} failed sign-ins, ${activity.rateLimited.total} rate limited, ${activity.activeSessions.browser + activity.activeSessions.cli} active sessions`,
      );
    }
    return lines.join("\n");
  }

  async function copyDiagnostics(): Promise<void> {
    diagnosticsCopied.value = await copyText(diagnosticsText());
    if (diagnosticsCopied.value) {
      setTimeout(() => (diagnosticsCopied.value = false), 2000);
    }
  }

  // The first reading comes with the query; the poll runs while the page is
  // open and skips a tick when a request is already out.
  let timer: ReturnType<typeof setInterval> | null = null;
  onMounted(() => {
    timer = setInterval(() => {
      if (!loading.value && !running.value) void load();
    }, POLL_INTERVAL_MS);
  });
  onUnmounted(() => {
    if (timer) clearInterval(timer);
  });

  /** Root takes another copy of the key that every backup needs. */
  async function downloadMasterKey(): Promise<void> {
    if (!isRoot.value) return;
    downloadingMasterKey.value = true;
    masterKeyMessage.value = null;
    try {
      await runWithReauth(async () => {
        saveBlob(await downloadMasterKeyBlob(), "master.key");
      });
      masterKeyMessage.value = {
        text: "Master key downloaded. Store it apart from your backups.",
        tone: "ok",
      };
    } catch (cause: unknown) {
      if (isAbortError(cause)) return;
      masterKeyMessage.value = {
        text: errorMessage(cause, "Could not download the master key."),
        tone: "crit",
      };
    } finally {
      downloadingMasterKey.value = false;
    }
  }

  async function performReset(): Promise<void> {
    if (!canReset.value) return;
    resetLoading.value = true;
    resetError.value = null;
    try {
      await factoryReset(
        resetPassword.value,
        resetConfirmation.value,
        resetAcknowledged.value,
      );
      resetComplete.value = true;
      auth.clearLocalSession();
      await router.push({ name: "setup" });
    } catch {
      resetError.value =
        "Factory reset was not completed. Check the password and the confirmation.";
    } finally {
      resetLoading.value = false;
    }
  }

  return {
    status,
    loading,
    loadError,
    staleSince,
    load,
    running,
    runError,
    runChecks,
    hasChecks,
    checks,
    attention,
    passed,
    showPassed,
    banner,
    serverLine,
    releaseBadge,
    healthCards,
    activityTiles,
    fixAction,
    runFix,
    severityLabel,
    severityTone,
    diagnosticsText,
    diagnosticsCopied,
    copyDiagnostics,
    counts,
    bootTime,
    formattedUptime,
    formattedObservedAt,
    relativeObservedAt,
    relativeBootTime,
    isRoot,
    downloadingMasterKey,
    masterKeyMessage,
    downloadMasterKey,
    resetOpen,
    resetPassword,
    resetConfirmation,
    resetAcknowledged,
    resetLoading,
    resetError,
    resetComplete,
    canReset,
    performReset,
  };
}

export type InstanceController = ReturnType<typeof useInstanceController>;
