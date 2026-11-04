import { computed } from "vue";
import { useQuery } from "@pinia/colada";
import { publicAuthMethodsQuery } from "~/queries/auth";

/**
 * Sign-in help: where "Forgot password?" goes. A self-hosted server may
 * not send email, so the page says who can actually help, and names the
 * single sign-on provider when there is one. It never names admins before
 * sign-in.
 *
 * The sign-in methods come from the same entry as the login page, so going
 * back and forth asks the server once.
 */
export function useLoginHelpController() {
  const host = window.location.host;
  const methods = useQuery(publicAuthMethodsQuery);
  // Without the provider's name the page still says who can help.
  const ssoName = computed(() => methods.data.value?.sso?.name ?? null);

  /** Asks again, if the last answer is stale. */
  async function load(): Promise<void> {
    await methods.refresh();
  }

  return { host, ssoName, load };
}
