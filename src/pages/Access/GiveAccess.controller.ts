import { computed, onMounted, ref, watch } from "vue";
import { useRoute, useRouter, type RouteLocationRaw } from "vue-router";
import { useQueryCache } from "@pinia/colada";
import { LEVELS, reaches, setGrant } from "~/services/access.api";
import type {
  Grant,
  Level,
  PrincipalType,
  ScopeType,
} from "~/services/access.api";
import type { Environment, Project } from "~/services";
import { errorMessage } from "~/services/api-errors";
import { useReauthentication } from "~/composable/reauthentication";
import { useAuthStore } from "~/stores/auth.store";
import {
  groupQuery,
  groupsQuery,
  projectAccessQuery,
  userAccessQuery,
} from "~/queries/access";
import { readQuery } from "~/queries/helpers";
import { useInvalidate } from "~/queries/invalidate";
import { environmentsQuery, projectsQuery } from "~/queries/projects";
import { userQuery, usersQuery } from "~/queries/users";
import type { User } from "~/services/users.api";
import { pages } from "~/navigation/access-map";
import type { Breadcrumb } from "~/navigation/breadcrumb";
import { formatLevel, formatList } from "~/utils/format";
import {
  grantSentence,
  levelChoices,
  type GrantPrincipal,
} from "~/utils/level";

/**
 * Who the page is for: a person, AI agent or group picked before it opened
 * (they choose a project), or a project (you choose who).
 */
export type GiveAccessKind = "user" | "agent" | "group" | "project";
export type GrantWhere = "project" | "environment";

const KIND_FOR_ROUTE: Record<string, GiveAccessKind> = {
  "user-access-give": "user",
  "agent-access-give": "agent",
  "group-access-give": "group",
  "project-access-give": "project",
};

/** The list each kind of page opens from, first in the breadcrumb. */
const SECTION_FOR_KIND = {
  user: "users",
  agent: "agents",
  group: "groups",
  project: "projects",
} as const satisfies Record<GiveAccessKind, keyof typeof pages>;

export interface ProjectChoice {
  project: Project;
  environmentCount: number;
  /** The level they hold on the whole project already, if any. */
  current: Level | null;
}

export interface EnvironmentChoice {
  environment: Environment;
  /** The level they hold on this environment already, if any. */
  current: Level | null;
}

/** One grant the page will save. */
export interface PlannedGrant {
  scopeType: ScopeType;
  scopeId: string;
  /** `payments` or `payments / staging`. */
  label: string;
  /** The level it replaces, when they already hold a grant there. */
  replaces: Level | null;
}

/** Someone you can give access to, on the page opened from a project. */
export interface PrincipalChoice {
  type: PrincipalType;
  id: string;
  /** An email, an agent's name, or a group's name. */
  label: string;
  /** Where the picker lists them. */
  group: "groups" | "people" | "agents";
  /** Their role, or how many people a group has. */
  detail: string;
  /** The level they hold on the whole project already, if any. */
  current: Level | null;
  /** How many of its environments they hold a grant on. */
  environmentGrants: number;
}

/** "4 environments · has Write now" under a project's name. */
export function projectMeta(choice: ProjectChoice): string {
  return [
    `${choice.environmentCount} ${choice.environmentCount === 1 ? "environment" : "environments"}`,
    choice.current ? `has ${formatLevel(choice.current)} now` : "",
  ]
    .filter(Boolean)
    .join(" · ");
}

/** "Member · has Write now" under a person's, agent's or group's name. */
export function principalMeta(choice: PrincipalChoice): string {
  const held = choice.current
    ? `has ${formatLevel(choice.current)} now`
    : choice.environmentGrants > 0
      ? `has ${choice.environmentGrants} ${choice.environmentGrants === 1 ? "environment" : "environments"}`
      : "";
  return [choice.detail, held].filter(Boolean).join(" · ");
}

export function principalKey(who: { type: PrincipalType; id: string }) {
  return `${who.type}:${who.id}`;
}

function memberCount(count: number): string {
  return `${count} ${count === 1 ? "member" : "members"}`;
}

/** The lower of two levels; an unknown one doesn't limit. */
function lower(a: Level | undefined, b: Level | undefined): Level | undefined {
  if (a === undefined) return b;
  if (b === undefined) return a;
  return LEVELS.indexOf(a) <= LEVELS.indexOf(b) ? a : b;
}

/** The higher of two levels; an unknown one counts as no access. */
function higher(a: Level | undefined, b: Level | undefined): Level {
  const left = a ?? "none";
  const right = b ?? "none";
  return LEVELS.indexOf(left) >= LEVELS.indexOf(right) ? left : right;
}

/**
 * Give access page. From a person's, AI agent's or group's Access tab you
 * pick a project you manage; from a project's Access tab you pick who.
 * Then the whole project or some of its environments, and one level. Each
 * place becomes one grant; a grant they already hold there is replaced.
 *
 * The server checks it all again: you need Project manager on the project,
 * and nobody can be given more than their role allows. The owner of an AI
 * agent is the exception: they give their own agent access anywhere they
 * reach, up to their own level there.
 */
export function useGiveAccessController() {
  const route = useRoute();
  const router = useRouter();
  const auth = useAuthStore();
  const cache = useQueryCache();
  const invalidate = useInvalidate();
  const { runWithReauth } = useReauthentication();

  const kind = computed<GiveAccessKind>(
    () => KIND_FOR_ROUTE[String(route.name)] ?? "user",
  );
  const principalId = computed(() =>
    String(
      route.params.userId ?? route.params.agentId ?? route.params.groupId ?? "",
    ),
  );
  const projectRef = computed(() => String(route.params.projectRef ?? ""));

  const principal = ref<GrantPrincipal | null>(null);
  /** The AI agent's owner, when the page is for an agent. */
  const agentOwnerId = ref<string | null>(null);
  /**
   * Grants held directly, not through a group: the principal's own, or,
   * from a project, everyone's on it.
   */
  const grants = ref<Grant[]>([]);
  const projects = ref<Project[] | null>(null);
  const environments = ref<Environment[]>([]);
  const loadError = ref<string | null>(null);

  /** From a project: who you can pick, and who is picked. */
  const candidates = ref<
    Omit<PrincipalChoice, "current" | "environmentGrants">[]
  >([]);
  const chosenKey = ref("");
  const principalFilter = ref("");

  const projectFilter = ref("");
  const environmentFilter = ref("");
  const projectId = ref("");
  const where = ref<GrantWhere>("project");
  const environmentIds = ref<string[]>([]);
  const level = ref<Level>("read");
  const saving = ref(false);
  const error = ref<string | null>(null);

  async function loadPrincipal(fresh = false): Promise<void> {
    const id = principalId.value;
    if (kind.value === "group") {
      const detail = await readQuery(cache, groupQuery(id), { fresh });
      principal.value = { type: "group", id, label: detail.group.name };
      grants.value = detail.grants.filter(
        (grant) => grant.principalType === "group" && grant.principalId === id,
      );
      return;
    }
    const [access, account] = await Promise.all([
      readQuery(cache, userAccessQuery(id), { fresh }),
      kind.value === "agent"
        ? readQuery(cache, userQuery(id), { fresh }).catch(() => null)
        : Promise.resolve(null),
    ]);
    agentOwnerId.value = account?.ownerId ?? null;
    principal.value = {
      type: "user",
      id,
      label: access.email,
      maxLevel: access.maxLevel,
    };
    grants.value = access.grants.filter(
      (grant) => grant.principalType === "user" && grant.principalId === id,
    );
  }

  /** From a project: its environments and every grant on it. */
  async function loadProject(fresh = false): Promise<void> {
    const access = await readQuery(
      cache,
      projectAccessQuery(projectRef.value),
      { fresh },
    );
    projects.value = [access.project];
    environments.value = access.environments;
    grants.value = access.grants;
    projectId.value = access.project.id;
  }

  /**
   * Whether the server lets you give this account access: people with
   * users.view, anyone's AI agent with agents.manage, and your own agents.
   */
  function grantable(user: User): boolean {
    // Without RBAC only AI agents take new access.
    if (user.role !== "ai_agent")
      return auth.can("users.view") && auth.hasFeature("rbac");
    return (
      auth.can("agents.manage") ||
      (auth.can("agents.view") && user.ownerId === auth.session?.adminId)
    );
  }

  /**
   * Groups, people and AI agents to pick from. Each list needs its own
   * capability, so a project manager without them sees only what they may
   * give access to. Root reaches every project already, and you can't give
   * yourself access.
   */
  async function loadPrincipals(): Promise<void> {
    const [users, groups] = await Promise.allSettled([
      auth.can("users.view", "agents.view", "agents.manage")
        ? readQuery(cache, usersQuery)
        : Promise.resolve([]),
      auth.can("users.view", "groups.manage") && auth.hasFeature("rbac")
        ? readQuery(cache, groupsQuery)
        : Promise.resolve([]),
    ]);
    const byName = (a: { label: string }, b: { label: string }) =>
      a.label.localeCompare(b.label);
    candidates.value = [
      ...(groups.status === "fulfilled" ? groups.value : [])
        .map((group) => ({
          type: "group" as const,
          id: group.id,
          label: group.name,
          group: "groups" as const,
          detail: memberCount(group.memberCount),
        }))
        .sort(byName),
      ...(users.status === "fulfilled" ? users.value : [])
        .filter(
          (user) =>
            user.role !== "root" &&
            user.id !== auth.session?.adminId &&
            grantable(user),
        )
        .map((user) => ({
          type: "user" as const,
          id: user.id,
          label: user.email,
          group:
            user.role === "ai_agent"
              ? ("agents" as const)
              : ("people" as const),
          detail: user.roleName ?? "",
        }))
        .sort(
          (a, b) =>
            Number(a.group === "agents") - Number(b.group === "agents") ||
            byName(a, b),
        ),
    ];
  }

  /** The candidates with what each holds on the project already. */
  const principals = computed<PrincipalChoice[]>(() =>
    candidates.value.map((candidate) => {
      const theirs = grants.value.filter(
        (grant) =>
          grant.principalType === candidate.type &&
          grant.principalId === candidate.id,
      );
      return {
        ...candidate,
        current:
          theirs.find((grant) => grant.scopeType === "project")?.level ?? null,
        environmentGrants: theirs.filter(
          (grant) => grant.scopeType === "environment",
        ).length,
      };
    }),
  );

  async function load(): Promise<void> {
    loadError.value = null;
    principal.value = null;
    projects.value = null;
    try {
      if (kind.value === "project") {
        await Promise.all([loadProject(), loadPrincipals()]);
        return;
      }
      // The form is filled once from the shared lists; a visit soon after
      // another screen loaded them needs no request.
      const [, projectList, environmentList] = await Promise.all([
        loadPrincipal(),
        readQuery(cache, projectsQuery),
        readQuery(cache, environmentsQuery(null)),
      ]);
      projects.value = projectList;
      environments.value = environmentList;
      // With one project to choose from, choose it.
      if (manageable.value.length === 1)
        projectId.value = manageable.value[0]!.project.id;
    } catch (cause) {
      loadError.value = errorMessage(
        cause,
        kind.value === "project"
          ? "Could not load who has access to this project."
          : kind.value === "group"
            ? "Could not load the group."
            : "Could not load their access.",
      );
    }
  }

  onMounted(load);

  const visiblePrincipals = computed(() => {
    const needle = principalFilter.value.trim().toLowerCase();
    return needle
      ? principals.value.filter((choice) =>
          choice.label.toLowerCase().includes(needle),
        )
      : principals.value;
  });

  /**
   * From a project: picks who gets access. A person's or agent's role cap
   * loads first, so the levels never offer what the server would refuse.
   */
  async function choosePrincipal(key: string): Promise<void> {
    if (chosenKey.value === key) return;
    chosenKey.value = key;
    principalFilter.value = "";
    principal.value = null;
    error.value = null;
    const choice = principals.value.find(
      (candidate) => principalKey(candidate) === key,
    );
    if (!choice) return;
    if (choice.type === "group") {
      principal.value = { type: "group", id: choice.id, label: choice.label };
      return;
    }
    try {
      const access = await readQuery(cache, userAccessQuery(choice.id));
      // A later pick wins.
      if (chosenKey.value !== key) return;
      principal.value = {
        type: "user",
        id: choice.id,
        label: choice.label,
        maxLevel: access.maxLevel,
      };
    } catch (cause) {
      if (chosenKey.value !== key) return;
      error.value = errorMessage(cause, "Could not load their role.");
    }
  }

  function currentOn(scopeType: ScopeType, scopeId: string): Level | null {
    const who = principal.value;
    if (!who) return null;
    return (
      grants.value.find(
        (grant) =>
          grant.principalType === who.type &&
          grant.principalId === who.id &&
          grant.scopeType === scopeType &&
          grant.scopeId === scopeId,
      )?.level ?? null
    );
  }

  /**
   * You own this AI agent and may run your agents, so you give it access
   * wherever you reach, never above your own level there.
   */
  const forOwnAgent = computed(
    () =>
      kind.value === "agent" &&
      auth.can("agents.view") &&
      agentOwnerId.value !== null &&
      agentOwnerId.value === auth.session?.adminId,
  );

  /**
   * Projects you can give access in: the ones you manage, or for your own
   * agent, every one you reach.
   */
  const manageable = computed<ProjectChoice[]>(() =>
    (projects.value ?? [])
      .filter((project) =>
        forOwnAgent.value
          ? reaches(project.level ?? "project_manager", "metadata")
          : reaches(project.level ?? "project_manager", "project_manager"),
      )
      .sort((a, b) => a.name.localeCompare(b.name))
      .map((project) => ({
        project,
        environmentCount: environments.value.filter(
          (environment) => environment.projectId === project.id,
        ).length,
        current: currentOn("project", project.id),
      })),
  );

  const visibleProjects = computed(() => {
    const needle = projectFilter.value.trim().toLowerCase();
    return needle
      ? manageable.value.filter((choice) =>
          choice.project.name.toLowerCase().includes(needle),
        )
      : manageable.value;
  });

  const project = computed(
    () =>
      manageable.value.find((choice) => choice.project.id === projectId.value)
        ?.project ?? null,
  );

  const environmentChoices = computed<EnvironmentChoice[]>(() =>
    environments.value
      .filter((environment) => environment.projectId === projectId.value)
      .sort((a, b) => a.name.localeCompare(b.name))
      .map((environment) => ({
        environment,
        current: currentOn("environment", environment.id),
      })),
  );

  const visibleEnvironments = computed(() => {
    const needle = environmentFilter.value.trim().toLowerCase();
    return needle
      ? environmentChoices.value.filter((choice) =>
          choice.environment.name.toLowerCase().includes(needle),
        )
      : environmentChoices.value;
  });

  function chooseProject(id: string): void {
    if (projectId.value === id) return;
    projectId.value = id;
    environmentIds.value = [];
    environmentFilter.value = "";
  }

  function toggleEnvironment(id: string, on: boolean): void {
    const rest = environmentIds.value.filter((candidate) => candidate !== id);
    environmentIds.value = on ? [...rest, id] : rest;
  }

  // Keep the level valid when the scope changes what is allowed.
  watch(where, (scope) => {
    // Project manager only exists on a whole project.
    if (scope === "environment" && level.value === "project_manager")
      level.value = "manage";
    if (scope === "project" && level.value === "none") level.value = "metadata";
  });

  /**
   * For your own agent, the most you can give here: your level on the
   * project, or on the environments chosen (the lowest of them binds).
   */
  const ownerCap = computed<Level | undefined>(() => {
    if (!forOwnAgent.value) return undefined;
    if (where.value === "project") return project.value?.level;
    const inProject = environmentChoices.value.map(
      (choice) => choice.environment,
    );
    const chosen = inProject.filter((environment) =>
      environmentIds.value.includes(environment.id),
    );
    return chosen.length > 0
      ? chosen.reduce<Level | undefined>(
          (cap, environment) => lower(cap, environment.level ?? "none"),
          undefined,
        )
      : inProject.reduce<Level>(
          (cap, environment) => higher(cap, environment.level),
          "none",
        );
  });

  const levels = computed(() =>
    levelChoices(where.value, lower(principal.value?.maxLevel, ownerCap.value)),
  );

  // Start from a level their role allows. Once their cap is known, or the
  // scope changes, a level above it drops to the highest one allowed.
  watch(levels, (choices) => {
    if (!choices.find((choice) => choice.level === level.value)?.disabled)
      return;
    const allowed = choices
      .filter((choice) => !choice.disabled && choice.level !== "none")
      .map((choice) => choice.level);
    const highest = allowed.reduce<Level | null>(
      (best, candidate) =>
        best === null || LEVELS.indexOf(candidate) > LEVELS.indexOf(best)
          ? candidate
          : best,
      null,
    );
    if (highest) level.value = highest;
  });

  /** What saving would write, one entry per grant. */
  const planned = computed<PlannedGrant[]>(() => {
    const chosen = project.value;
    if (!chosen) return [];
    if (where.value === "project")
      return [
        {
          scopeType: "project",
          scopeId: chosen.id,
          label: chosen.name,
          replaces: currentOn("project", chosen.id),
        },
      ];
    return environmentChoices.value
      .filter((choice) => environmentIds.value.includes(choice.environment.id))
      .map((choice) => ({
        scopeType: "environment" as const,
        scopeId: choice.environment.id,
        label: `${chosen.name} / ${choice.environment.name}`,
        replaces: choice.current,
      }));
  });

  const summary = computed(() => {
    const chosen = project.value;
    const who = principal.value;
    if (!who || !chosen) return "";
    const target =
      where.value === "project"
        ? `all of ${chosen.name}`
        : `${chosen.name} / ${formatList(
            planned.value.map((grant) => grant.label.split(" / ")[1]!),
          )}`;
    return grantSentence(who.label, who.type === "group", level.value, target);
  });

  /**
   * A whole-project grant below Project manager skips protected
   * environments; say which ones it leaves out.
   */
  const protectedNote = computed(() => {
    if (where.value !== "project" || level.value === "project_manager")
      return "";
    const names = environmentChoices.value
      .filter((choice) => choice.environment.protected)
      .map((choice) => choice.environment.name);
    if (names.length === 0) return "";
    return `${formatList(names)} ${names.length === 1 ? "is" : "are"} protected, so this grant doesn't reach ${names.length === 1 ? "it" : "them"}. Give access to ${names.length === 1 ? "it" : "them"} on its own.`;
  });

  const backTo = computed<RouteLocationRaw>(() => {
    switch (kind.value) {
      case "project":
        return {
          name: "project-access",
          params: { projectRef: projectRef.value },
        };
      case "agent":
        return { name: "agent", params: { agentId: principalId.value } };
      case "group":
        return { name: "group-access", params: { groupId: principalId.value } };
      default:
        return { name: "user-access", params: { userId: principalId.value } };
    }
  });

  /** The project, or the person, agent or group, the page opened from. */
  const parentLabel = computed(() =>
    kind.value === "project"
      ? (projects.value?.[0]?.name ?? "")
      : (principal.value?.label ?? ""),
  );

  /**
   * "Users / maya@dopbase.com", then the page's own title. The name is left
   * out until it loads, so the breadcrumb never shows an empty link.
   */
  const breadcrumbs = computed<Breadcrumb[]>(() => {
    const section = SECTION_FOR_KIND[kind.value];
    const trail: Breadcrumb[] = [
      { label: pages[section].label, to: { name: section } },
    ];
    if (parentLabel.value)
      trail.push({ label: parentLabel.value, to: backTo.value });
    return trail;
  });

  async function submit(): Promise<boolean> {
    error.value = null;
    const who = principal.value;
    if (!who) {
      error.value = "Choose who gets access.";
      return false;
    }
    if (!project.value) {
      error.value = "Choose a project.";
      return false;
    }
    if (planned.value.length === 0) {
      error.value = "Choose at least one environment.";
      return false;
    }
    if (levels.value.find((choice) => choice.level === level.value)?.disabled) {
      const yours = ownerCap.value;
      error.value =
        yours !== undefined &&
        lower(who.maxLevel, yours) === yours &&
        yours !== who.maxLevel
          ? `You can give up to ${formatLevel(yours)} here, your own level.`
          : `Their role allows up to ${formatLevel(who.maxLevel ?? "none")}.`;
      return false;
    }
    saving.value = true;
    const todo = [...planned.value];
    let saved = 0;
    try {
      // One grant per place. Setting a grant replaces the one there, so
      // retrying after a sign-in prompt is safe.
      await runWithReauth(async () => {
        for (const grant of todo.slice(saved)) {
          await setGrant({
            principalType: who.type,
            principalId: who.id,
            scopeType: grant.scopeType,
            scope: grant.scopeId,
            level: level.value,
          });
          saved += 1;
        }
      });
    } catch (cause) {
      const reason = errorMessage(cause, "Could not give that access.");
      error.value =
        saved > 0
          ? `Saved ${saved} of ${todo.length}. ${todo[saved]!.label} failed: ${reason}`
          : reason;
      if (saved > 0) {
        void invalidate.accessChanged();
        // Show what is held now, so a second try only redoes the rest.
        await (
          kind.value === "project" ? loadProject(true) : loadPrincipal(true)
        ).catch(() => undefined);
      }
      return false;
    } finally {
      saving.value = false;
    }
    // Every screen showing access, and the lists it decides, refresh.
    void invalidate.accessChanged();
    try {
      await router.push(backTo.value);
    } catch {
      // Every grant is in; only going back failed.
      error.value =
        kind.value === "project"
          ? "Access given, but this page couldn't go back. Open the project's Access tab to see it."
          : "Access given, but this page couldn't go back. Open their Access tab to see it.";
    }
    return true;
  }

  /** Whether the page has what it needs to show the form. */
  const ready = computed(
    () =>
      projects.value !== null &&
      (kind.value === "project" || principal.value !== null),
  );

  function cancel(): void {
    void router.push(backTo.value);
  }

  return {
    kind,
    forOwnAgent,
    ownerCap,
    principal,
    principals,
    visiblePrincipals,
    principalFilter,
    chosenKey,
    choosePrincipal,
    projects,
    ready,
    loadError,
    load,
    projectFilter,
    manageable,
    visibleProjects,
    projectId,
    project,
    chooseProject,
    where,
    environmentChoices,
    environmentFilter,
    visibleEnvironments,
    environmentIds,
    toggleEnvironment,
    level,
    levels,
    planned,
    summary,
    protectedNote,
    backTo,
    parentLabel,
    breadcrumbs,
    saving,
    error,
    submit,
    cancel,
  };
}

export type GiveAccessController = ReturnType<typeof useGiveAccessController>;
