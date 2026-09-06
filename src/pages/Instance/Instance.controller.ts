import { computed, onMounted, ref } from "vue";
import { useRouter } from "vue-router";
import { useAuthStore } from "~/stores/auth.store";
import { factoryReset, fetchFactoryResetPreview, fetchStatus } from "~/services/instance.api";
import type { StatusResponse } from "~/services/instance.api";
import { formatDateTime, formatDuration, formatRelativeTime } from "~/utils/format";

/**
 * Instance screen controller: safe, read-only server status.
 *
 * The API deliberately exposes no database paths, key paths, or private
 * configuration — and the UI never offers configuration edits, because
 * server settings are restart-only.
 */
export function useInstanceController() {
  const router = useRouter();
  const auth = useAuthStore();
  const status = ref<StatusResponse | null>(null);
  const loading = ref(false);
  const loadError = ref<string | null>(null);
  const resetPreview = ref<Awaited<ReturnType<typeof fetchFactoryResetPreview>> | null>(null);
  const resetPassword = ref("");
  const resetConfirmation = ref("");
  const resetAcknowledged = ref(false);
  const resetLoading = ref(false);
  const resetError = ref<string | null>(null);
  const resetComplete = ref(false);

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

  async function loadResetPreview(): Promise<void> {
    try {
      resetPreview.value = await fetchFactoryResetPreview();
    } catch {
      resetError.value = "Only the root administrator can load reset details.";
    }
  }

  async function performReset(): Promise<void> {
    resetLoading.value = true;
    resetError.value = null;
    try {
      await factoryReset(resetPassword.value, resetConfirmation.value, resetAcknowledged.value);
      resetComplete.value = true;
      auth.clearLocalSession();
      await router.push({ name: "setup" });
    } catch {
      resetError.value = "Factory reset was not completed. Verify the password and confirmation.";
    } finally {
      resetLoading.value = false;
    }
  }

  function healthTone(value: string): "ok" | "crit" | "neutral" {
    if (value === "healthy" || value === "available") return "ok";
    if (value === "unhealthy" || value === "unavailable") return "crit";
    return "neutral";
  }

  return {
    status,
    loading,
    loadError,
    load,
    healthTone,
    bootTime,
    formattedUptime,
    formattedObservedAt,
    relativeObservedAt,
    relativeBootTime,
    resetPreview,
    resetPassword,
    resetConfirmation,
    resetAcknowledged,
    resetLoading,
    resetError,
    resetComplete,
    loadResetPreview,
    performReset,
    isRoot: auth.isRoot,
  };
}

export type InstanceController = ReturnType<typeof useInstanceController>;
