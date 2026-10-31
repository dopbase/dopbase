import { computed, onMounted, ref } from "vue";
import { useRouter } from "vue-router";
import { useAuthStore } from "~/stores/auth.store";
import { useReauthentication } from "~/composable/reauthentication";
import { factoryReset, fetchStatus } from "~/services/instance.api";
import type { StatusResponse } from "~/services/instance.api";
import { downloadMasterKeyBlob } from "~/services/backups.api";
import { errorMessage, isAbortError } from "~/services/api-errors";
import { saveBlob } from "~/utils/download";
import {
  formatDateTime,
  formatDuration,
  formatRelativeTime,
} from "~/utils/format";

/** What the user types to confirm a factory reset. */
export const RESET_PHRASE = "FACTORY RESET";

export type HealthTone = "ok" | "crit" | "neutral";

/**
 * Status screen controller: safe, read-only server status, plus the
 * root-only master key download and factory reset.
 *
 * The API deliberately exposes no database paths, key paths, or private
 * configuration — and the UI never offers configuration edits, because
 * server settings are restart-only.
 */
export function useInstanceController() {
  const router = useRouter();
  const auth = useAuthStore();
  const { runWithReauth } = useReauthentication();
  const status = ref<StatusResponse | null>(null);
  const loading = ref(false);
  const loadError = ref<string | null>(null);

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

  /** The "What's stored" grid, in the order the status API counts them. */
  const counts = computed(() => {
    const current = status.value;
    if (!current) return [];
    return [
      { label: "Projects", value: current.projects },
      { label: "Environments", value: current.environments },
      { label: "Secrets", value: current.secrets },
      { label: "People", value: current.humanUsers },
      { label: "AI agents", value: current.aiAgents },
      { label: "Runner tokens", value: current.activeRunnerTokens },
      { label: "AI agent tokens", value: current.activeAgentTokens },
      { label: "Backups", value: current.backups },
    ];
  });

  async function load(): Promise<void> {
    loading.value = true;
    loadError.value = null;
    try {
      status.value = await fetchStatus();
    } catch {
      loadError.value = "Could not load the instance status.";
    } finally {
      loading.value = false;
    }
  }

  onMounted(load);

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

  function healthTone(value: string): HealthTone {
    if (value === "healthy" || value === "available") return "ok";
    if (value === "unhealthy" || value === "unavailable") return "crit";
    return "neutral";
  }

  /** "healthy" reads as "Healthy" in a stat card. */
  function healthLabel(value: string): string {
    return value ? value.charAt(0).toUpperCase() + value.slice(1) : "Unknown";
  }

  /** The short chip beside a health value. */
  function healthBadge(value: string): string {
    const tone = healthTone(value);
    if (tone === "ok") return "ok";
    if (tone === "crit") return "down";
    return "unknown";
  }

  return {
    status,
    loading,
    loadError,
    load,
    healthTone,
    healthLabel,
    healthBadge,
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
