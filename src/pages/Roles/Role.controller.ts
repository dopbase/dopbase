import { computed, onUnmounted, ref, watch } from "vue";
import { useRoute, useRouter } from "vue-router";
import { createRole, fetchRoles, updateRole } from "~/services/roles.api";
import type {
  Role,
  RoleSettings,
  SignIn,
  TokenScope,
} from "~/services/roles.api";
import type { Capability, Level } from "~/services/access.api";
import { errorMessage } from "~/services/api-errors";
import { useReauthentication } from "~/composable/reauthentication";
import { formatLevel } from "~/utils/format";
import { PROJECT_LEVELS } from "~/utils/level";
import { ROLE_CAPABILITIES } from "./Roles.controller";

/** Levels a personal token may be given. */
export const TOKEN_LEVELS: Level[] = ["metadata", "read"];
/** Longest-lifetime choices, in days. */
export const TOKEN_LIFETIMES = [30, 90, 365] as const;

export const MAX_LEVEL_OPTIONS = PROJECT_LEVELS.map((level) => ({
  value: level,
  label: formatLevel(level),
}));
export const TOKEN_LEVEL_OPTIONS = TOKEN_LEVELS.map((level) => ({
  value: level,
  label: formatLevel(level),
}));
export const LIFETIME_OPTIONS = TOKEN_LIFETIMES.map((days) => ({
  value: String(days),
  label: `${days} days`,
}));

/** The closest lifetime choice at or under `days`. */
function lifetimeFor(days: number): string {
  const fits = TOKEN_LIFETIMES.filter((choice) => choice <= days);
  return String(fits.at(-1) ?? TOKEN_LIFETIMES[0]);
}

/**
 * Role editor at /roles/new and /roles/:roleId. `?from=<id>` starts a new
 * role from another one's settings (Duplicate). A built-in role opens read
 * only. Sign-in is picked once, when the role is created.
 */
export function useRoleController() {
  const route = useRoute();
  const router = useRouter();
  const { runWithReauth } = useReauthentication();

  const roleId = computed(() =>
    route.name === "role" ? String(route.params.roleId ?? "") : null,
  );
  const fromId = computed(() =>
    roleId.value === null && typeof route.query.from === "string"
      ? route.query.from
      : null,
  );

  const roles = ref<Role[] | null>(null);
  const loadError = ref<string | null>(null);
  const notFound = ref(false);

  const name = ref("");
  const description = ref("");
  const maxLevel = ref<Level>("read");
  const capabilities = ref<Capability[]>(["tokens.personal"]);
  const signIn = ref<SignIn>("interactive");
  const tokenScope = ref<Exclude<TokenScope, "none">>("data");
  const tokenMaxLevel = ref<Level>("read");
  const tokenLifetime = ref("365");
  const tokenDefaultTtlDays = ref(30);

  const saving = ref(false);
  const nameError = ref<string | null>(null);
  const saveError = ref<string | null>(null);
  let version = 0;

  /** The role being edited or viewed; null for a new one. */
  const role = computed(() =>
    roleId.value
      ? (roles.value?.find((candidate) => candidate.id === roleId.value) ??
        null)
      : null,
  );
  const source = computed(() =>
    fromId.value
      ? (roles.value?.find((candidate) => candidate.id === fromId.value) ??
        null)
      : null,
  );
  const isNew = computed(() => roleId.value === null);
  const readOnly = computed(() => role.value?.builtin === true);

  function fill(from: Role | null, copy: boolean): void {
    name.value = from ? (copy ? `${from.name} copy` : from.name) : "";
    description.value = from?.description ?? "";
    maxLevel.value = from?.maxLevel ?? "read";
    capabilities.value = from
      ? from.capabilities.filter((capability) =>
          ROLE_CAPABILITIES.includes(capability),
        )
      : ["tokens.personal"];
    signIn.value = from?.signIn ?? "interactive";
    tokenScope.value = from?.tokenScope === "audit" ? "audit" : "data";
    tokenMaxLevel.value = TOKEN_LEVELS.includes(from?.tokenMaxLevel ?? "read")
      ? (from?.tokenMaxLevel ?? "read")
      : "read";
    tokenLifetime.value = lifetimeFor(from?.tokenMaxTtlDays ?? 365);
    tokenDefaultTtlDays.value = from?.tokenDefaultTtlDays ?? 30;
    nameError.value = null;
    saveError.value = null;
  }

  async function load(): Promise<void> {
    const current = ++version;
    loadError.value = null;
    notFound.value = false;
    try {
      const list = await fetchRoles();
      if (current !== version) return;
      roles.value = list;
      if (roleId.value) {
        notFound.value = role.value === null;
        fill(role.value, false);
      } else {
        fill(source.value, source.value !== null);
      }
    } catch (cause) {
      if (current !== version) return;
      loadError.value = errorMessage(cause, "Could not load roles.");
    }
  }

  watch([roleId, fromId], () => void load(), { immediate: true });
  onUnmounted(() => {
    version++;
  });

  const title = computed(() => {
    if (isNew.value)
      return source.value ? `New role from ${source.value.name}` : "New role";
    return role.value?.name ?? "Role";
  });

  const holdsTokens = computed(() =>
    capabilities.value.includes("tokens.personal"),
  );

  function has(capability: Capability): boolean {
    return capabilities.value.includes(capability);
  }

  function toggle(capability: Capability, on: boolean): void {
    if (readOnly.value) return;
    const next = new Set(capabilities.value);
    if (on) next.add(capability);
    else next.delete(capability);
    // Keep checklist order so the request reads the same each time.
    capabilities.value = ROLE_CAPABILITIES.filter((item) => next.has(item));
  }

  /** What the request sends. Token settings only matter with tokens.personal. */
  const settings = computed<RoleSettings>(() => {
    const lifetime = Number(tokenLifetime.value);
    return {
      name: name.value.trim(),
      description: description.value.trim(),
      capabilities: capabilities.value,
      maxLevel: maxLevel.value,
      tokenScope: holdsTokens.value ? tokenScope.value : "none",
      tokenMaxLevel: tokenMaxLevel.value,
      tokenMaxTtlDays: lifetime,
      tokenDefaultTtlDays: Math.min(tokenDefaultTtlDays.value, lifetime),
      ...(isNew.value ? { signIn: signIn.value } : {}),
    };
  });

  async function save(): Promise<boolean> {
    if (readOnly.value) return false;
    nameError.value = settings.value.name ? null : "Enter a role name.";
    if (nameError.value) return false;
    saving.value = true;
    saveError.value = null;
    try {
      let saved: Role | null = null;
      await runWithReauth(async () => {
        saved = roleId.value
          ? await updateRole(roleId.value, settings.value)
          : await createRole(settings.value);
      });
      if (!saved) return false;
      await router.push({ name: "roles" });
      return true;
    } catch (cause) {
      if (cause instanceof DOMException && cause.name === "AbortError") {
        return false;
      }
      saveError.value = errorMessage(cause, "Could not save the role.");
      return false;
    } finally {
      saving.value = false;
    }
  }

  function duplicate(): void {
    if (!role.value) return;
    void router.push({ name: "role-new", query: { from: role.value.id } });
  }

  return {
    roleId,
    roles,
    role,
    source,
    isNew,
    readOnly,
    loadError,
    notFound,
    load,
    title,
    name,
    description,
    maxLevel,
    capabilities,
    signIn,
    tokenScope,
    tokenMaxLevel,
    tokenLifetime,
    holdsTokens,
    has,
    toggle,
    settings,
    saving,
    nameError,
    saveError,
    save,
    duplicate,
  };
}
