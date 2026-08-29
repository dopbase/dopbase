import { onMounted, ref } from "vue";
import { fetchHealth } from "~/services/health.api";
import type { HealthResponse } from "~/services/health.api";

/**
 * Public server status for the auth screens' "instance seal" panel: the
 * endpoint and whether the server is reachable. Read-only and public —
 * safe to show before any authentication.
 */
export function useServerStatus() {
  const health = ref<HealthResponse | null>(null);
  const reachable = ref<boolean | null>(null);
  const endpoint = ref(`${window.location.origin}`);

  onMounted(async () => {
    try {
      health.value = await fetchHealth();
      reachable.value = true;
    } catch {
      reachable.value = false;
    }
  });

  return { health, reachable, endpoint };
}
