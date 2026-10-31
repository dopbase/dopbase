import { onMounted, ref } from "vue";
import { fetchAuthMethods } from "~/services/auth.api";

/**
 * Sign-in help: where "Forgot password?" goes. A self-hosted server may
 * not send email, so the page says who can actually help, and names the
 * single sign-on provider when there is one. It never names admins before
 * sign-in.
 */
export function useLoginHelpController() {
  const host = window.location.host;
  const ssoName = ref<string | null>(null);

  async function load(): Promise<void> {
    try {
      ssoName.value = (await fetchAuthMethods()).sso?.name ?? null;
    } catch {
      // Without the provider's name the page still says who can help.
      ssoName.value = null;
    }
  }

  onMounted(load);

  return { host, ssoName, load };
}
