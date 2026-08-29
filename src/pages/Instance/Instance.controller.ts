import { onMounted, ref } from "vue";
import { fetchInstanceStatus } from "~/services/instance.api";
import type { InstanceStatus } from "~/services/instance.api";

/**
 * Instance screen controller: safe, read-only server status.
 *
 * The API deliberately exposes no database paths, key paths, or private
 * configuration — and the UI never offers configuration edits, because
 * server settings are restart-only.
 */
export function useInstanceController() {
  const status = ref<InstanceStatus | null>(null);
  const loading = ref(false);
  const loadError = ref<string | null>(null);

  async function load(): Promise<void> {
    loading.value = true;
    loadError.value = null;
    try {
      status.value = await fetchInstanceStatus();
    } catch {
      loadError.value = "Could not load the instance status.";
    } finally {
      loading.value = false;
    }
  }

  onMounted(load);

  function healthTone(value: string): "ok" | "crit" | "neutral" {
    if (value === "healthy" || value === "available") return "ok";
    if (value === "unhealthy" || value === "unavailable") return "crit";
    return "neutral";
  }

  return { status, loading, loadError, load, healthTone };
}

export type InstanceController = ReturnType<typeof useInstanceController>;
