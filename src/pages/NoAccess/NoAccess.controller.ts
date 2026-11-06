import { computed } from "vue";
import { useRoute, useRouter } from "vue-router";
import { useAuthStore } from "~/stores/auth.store";
import { isPageName, pageFeature, pages } from "~/navigation/access-map";
import type { Feature } from "~/services/access.api";
import { formatRole } from "~/utils/format";

export const DOCS_URL = "https://docs.dopbase.com";

/**
 * Why a page is off, said without naming settings: the page links to the
 * docs, where the person running the server looks up how to turn it on.
 */
const FEATURE_OFF: Record<Feature, string> = {
  sso: "This feature is disabled on this server. Your server administrator can enable it by following the",
  rbac: "Groups require people management, which is disabled on this server. Your server administrator can enable it by following the",
  "rbac.roles":
    "Custom roles are disabled on this server. Your server administrator can enable them by following the",
};

/** Custom roles while people management, which they need, is off too. */
const ROLES_WITHOUT_RBAC =
  "Custom roles require people management. Your server administrator can enable them by following the";

/**
 * NoAccess controller. The router guard renders this screen at the address
 * the viewer asked for, so resolving that address again names the page
 * they can't open and the capabilities it needs. A page whose feature the
 * server has off says so instead, and `docsUrl` is set for the page to link.
 */
export function useNoAccessController() {
  const route = useRoute();
  const router = useRouter();
  const auth = useAuthStore();

  const page = computed(() => {
    const name = router.resolve(route.path).name;
    return isPageName(name) ? pages[name] : null;
  });

  /** The feature the page needs, when the server has it off. */
  const offFeature = computed(() => {
    const feature = pageFeature(router.resolve(route.path).name);
    return feature && !auth.hasFeature(feature) ? feature : null;
  });

  const title = computed(() => {
    if (offFeature.value && page.value)
      return `${page.value.label} is turned off`;
    return page.value
      ? `You don't have access to ${page.value.label}`
      : "You don't have access to this page";
  });

  const description = computed(() => {
    if (offFeature.value === "rbac.roles" && !auth.hasFeature("rbac"))
      return ROLES_WITHOUT_RBAC;
    if (offFeature.value) return FEATURE_OFF[offFeature.value];
    const role = auth.session
      ? auth.session.roleName || formatRole(auth.session.role)
      : "";
    const needed = page.value?.capabilities ?? [];
    const subject = page.value?.label ?? "This page";
    if (needed.length === 0 || !role) {
      return "Your role doesn't include this page. Ask root if you need it.";
    }
    if (needed.length === 1) {
      return `${subject} needs the ${needed[0]} capability, which your role (${role}) doesn't have. Ask root if you need it.`;
    }
    return `${subject} needs ${needed.join(" or ")}, and your role (${role}) has neither. Ask root if you need it.`;
  });

  /** Set while the page's feature is off, for the "how" link. */
  const docsUrl = computed(() => (offFeature.value ? DOCS_URL : null));

  return { title, description, docsUrl };
}
