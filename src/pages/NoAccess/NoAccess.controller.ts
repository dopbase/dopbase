import { computed } from "vue";
import { useRoute, useRouter } from "vue-router";
import { useAuthStore } from "~/stores/auth.store";
import { isPageName, pages } from "~/navigation/access-map";
import { formatRole } from "~/utils/format";

/**
 * NoAccess controller. The router guard renders this screen at the address
 * the viewer asked for, so resolving that address again names the page
 * they can't open and the capabilities it needs.
 */
export function useNoAccessController() {
  const route = useRoute();
  const router = useRouter();
  const auth = useAuthStore();

  const page = computed(() => {
    const name = router.resolve(route.path).name;
    return isPageName(name) ? pages[name] : null;
  });

  const title = computed(() =>
    page.value
      ? `You don't have access to ${page.value.label}`
      : "You don't have access to this page",
  );

  const description = computed(() => {
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
