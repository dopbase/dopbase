import { onMounted, ref } from "vue";
import { fetchHealth } from "~/services/health.api";
import type { HealthResponse } from "~/services/health.api";
import { useRequestScope } from "./request-scope";

/**
 * Public server status for the auth screens' "instance seal" panel: the
 * endpoint and whether the server is reachable. Read-only and public —
 * safe to show before any authentication.
 */
export function useServerStatus() {
  const health = ref<HealthResponse | null>(null);
  const reachable = ref<boolean | null>(null);
  const endpoint = ref(`${window.location.origin}`);
  const requestScope = useRequestScope();

  onMounted(async () => {
    const signal = requestScope.begin();
    try {
      const result = await fetchHealth(signal);
      if (!requestScope.isCurrent(signal)) return;
      health.value = result;
      reachable.value = true;
    } catch {
      if (!requestScope.isCurrent(signal)) return;
      reachable.value = false;
    }
  });

  return { health, reachable, endpoint };
}
