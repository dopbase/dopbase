import { computed } from "vue";
import { useRoute, useRouter } from "vue-router";
import { useAuthStore } from "~/stores/auth.store";
import { isPageName, pageFeature, pages } from "~/navigation/access-map";
import { FEATURE_SETTINGS } from "~/services/access.api";
import { formatRole } from "~/utils/format";

/**
 * NoAccess controller. The router guard renders this screen at the address
 * the viewer asked for, so resolving that address again names the page
 * they can't open and the capabilities it needs. A page whose feature the
 * server has off says which setting turns it on instead.
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
    if (offFeature.value) {
      return `This server doesn't use it. To turn it on, set ${FEATURE_SETTINGS[offFeature.value]}, then restart Dopbase.`;
    }
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

  return { title, description };
}
