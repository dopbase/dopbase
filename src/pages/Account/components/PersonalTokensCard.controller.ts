import { computed, onMounted, ref } from "vue";
import {
  createPersonalToken,
  listPersonalTokens,
  revokePersonalToken,
} from "~/services/personal-tokens.api";
import type {
  CreatedPersonalToken,
  PersonalToken,
} from "~/services/personal-tokens.api";
import { listProjects } from "~/services/projects.api";
import type { Project } from "~/services/projects.api";
import type { Level } from "~/services/access.api";
import { errorMessage } from "~/services/api-errors";
import { useReauthentication } from "~/composable/reauthentication";
import { useAuthStore } from "~/stores/auth.store";

export const TOKEN_LEVEL_OPTIONS = [
  { value: "metadata", label: "Names only - projects and secret keys" },
  { value: "read", label: "Read - also reveal values and run commands" },
  { value: "write", label: "Write - also change secrets" },
];

export const TOKEN_EXPIRY_OPTIONS = [
  { value: "1d", label: "1 day" },
  { value: "7d", label: "7 days" },
  { value: "30d", label: "30 days" },
  { value: "90d", label: "90 days" },
  { value: "365d", label: "1 year" },
];

/** The scope picker's value for every project you reach. */
export const ALL_PROJECTS = "";

/**
 * Personal tokens: credentials that act as you, for scripts and AI agents
 * on your machines. A token reaches at most what you reach, within its own
 * project and level, and always expires.
 */
export function usePersonalTokensCardController() {
  const auth = useAuthStore();
  const { runWithReauth } = useReauthentication();

  const tokens = ref<PersonalToken[] | null>(null);
  const projects = ref<Project[]>([]);
  const error = ref<string | null>(null);
  const created = ref<CreatedPersonalToken | null>(null);
  const visible = computed(() => !auth.isRoot && auth.can("tokens.personal"));
  /** Roles such as Auditor hold tokens that read the audit log only. */
  const auditOnly = computed(() => auth.session?.tokenScope === "audit");
  const active = computed(() =>
    (tokens.value ?? []).filter(
      (token) =>
        !token.revokedAt &&
        (!token.expiresAt || new Date(token.expiresAt) > new Date()),
    ),
  );

  async function load(): Promise<void> {
    if (!visible.value) return;
    error.value = null;
    try {
      const [mine, shared] = await Promise.all([
        listPersonalTokens(),
        auditOnly.value ? Promise.resolve([]) : listProjects(),
      ]);
      tokens.value = mine;
      projects.value = shared;
    } catch (cause) {
      tokens.value = [];
      error.value = errorMessage(cause, "Could not load your tokens.");
    }
  }

  async function create(
    name: string,
    level: Level,
    project: string,
    expiresIn: string,
  ): Promise<void> {
    await runWithReauth(async () => {
      created.value = await createPersonalToken(
        "me",
        auditOnly.value
          ? { name, scopeType: "audit", expiresIn }
          : {
              name,
              level,
              expiresIn,
              ...(project === ALL_PROJECTS
                ? { scopeType: "all" as const }
                : { scopeType: "project" as const, scope: project }),
            },
      );
    });
    await load();
  }

  async function revoke(token: PersonalToken): Promise<void> {
    error.value = null;
    try {
      await revokePersonalToken("me", token.id);
      await load();
    } catch (cause) {
      error.value = errorMessage(cause, "Could not revoke that token.");
    }
  }

  function acknowledge(): void {
    created.value = null;
  }

  onMounted(load);

  return {
    tokens,
    active,
    projects,
    error,
    created,
    visible,
    auditOnly,
    load,
    create,
    revoke,
    acknowledge,
  };
}
