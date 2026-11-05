import { LEVELS, reaches } from "~/services/access.api";
import type { Capability, Level } from "~/services/access.api";
import type { Role, SignIn, TokenScope } from "~/services/roles.api";
import { formatLevel, formatList } from "~/utils/format";
import { ROLE_CAPABILITIES } from "./Roles.controller";

/** What the summary reads from the role editor's form. */
export interface RoleDraft {
  name: string;
  capabilities: Capability[];
  maxLevel: Level;
  /** How holders sign in; people (`interactive`) when left out. */
  signIn?: SignIn;
  /** Token settings, which a role without tokens.personal leaves out. */
  tokenScope?: TokenScope;
  tokenMaxLevel?: Level;
  tokenMaxTtlDays?: number;
}

export interface RoleWarning {
  title: string;
  body: string;
}

export interface RoleSummary {
  maxLevel: Level;
  /** Capabilities holders get, out of those a role can hold. */
  held: number;
  total: number;
  /** "Tokens, own AI agents, audit for their projects". */
  heldList: string;
  /** "Read · 90 days", or "None". */
  tokens: string;
  tokensDetail: string;
  /** "Root and Admins", or "Only root". */
  givers: string;
  giversDetail: string;
  /** The nearest built-in role people sign in with, or null. */
  closest: Role | null;
  /** "+ audit.read, Write not Manage", or "Same as Member". */
  closestDetail: string;
  warnings: RoleWarning[];
  /** "Developer reaches Write and holds 3 of 15 capabilities." */
  sentence: string;
}

/** Short names for the list under "Holders can". */
const SHORT: Record<string, string> = {
  "users.view": "see people",
  "users.manage": "manage people",
  "groups.manage": "manage groups",
  "projects.create": "create projects",
  "audit.read": "audit for their projects",
  "audit.read_all": "the whole audit log",
  "audit.verify": "verify the audit log",
  "access.report": "who reaches projects",
  "tokens.personal": "tokens",
  "tokens.revoke_others": "revoke others' tokens",
  "agents.view": "own AI agents",
  "agents.view_all": "see every AI agent",
  "agents.manage": "manage any AI agent",
  "instance.view": "instance status",
};

/** Powers over other people's accounts and tokens, which deserve a warning. */
const WARNINGS: Partial<Record<Capability, RoleWarning>> = {
  "users.manage": {
    title: "Holders can manage people",
    body: "They can add people and give them roles within their own. Give it only to people you'd trust with Users.",
  },
  "agents.manage": {
    title: "Holders can manage any AI agent",
    body: "They can change, delete or hand over other people's agents within their own access.",
  },
  "tokens.revoke_others": {
    title: "Holders can revoke other people's tokens",
    body: "A revoked token stops scripts and CI that use it straight away.",
  },
};

/** The ones the editor lists, in checklist order. */
function holdable(capabilities: readonly Capability[]): Capability[] {
  return ROLE_CAPABILITIES.filter((capability) =>
    capabilities.includes(capability),
  );
}

function capitalize(text: string): string {
  return text.charAt(0).toUpperCase() + text.slice(1);
}

/** "Admin" → "Admins", for "Root and Admins". */
function plural(name: string): string {
  return name.endsWith("s") ? name : `${name}s`;
}

/**
 * What giving a role takes besides covering it: users.manage to give it to
 * people, agents.view (creating an agent) or agents.manage (changing one)
 * for a token-only role.
 */
function givingCapabilities(draft: RoleDraft): Capability[] {
  return draft.signIn === "token_only"
    ? ["agents.view", "agents.manage"]
    : ["users.manage"];
}

/**
 * Whether holders of `giver` may hand out the draft: they hold what giving
 * it takes, every capability is one they hold, and the draft reaches no
 * deeper. Mirrors the server's Access::includes and the users endpoints.
 */
function covers(giver: Role, draft: RoleDraft): boolean {
  return (
    givingCapabilities(draft).some((capability) =>
      giver.capabilities.includes(capability),
    ) &&
    draft.capabilities.every((capability) =>
      giver.capabilities.includes(capability),
    ) &&
    reaches(giver.maxLevel, draft.maxLevel)
  );
}

function tokenSummary(draft: RoleDraft): { value: string; detail: string } {
  if (
    !draft.tokenScope ||
    draft.tokenScope === "none" ||
    !draft.capabilities.includes("tokens.personal")
  )
    return { value: "None", detail: "No personal tokens" };
  const days = `${draft.tokenMaxTtlDays ?? 365} days`;
  if (draft.tokenScope === "audit")
    return { value: `Audit log · ${days}`, detail: "No project data" };
  return {
    value: `${formatLevel(draft.tokenMaxLevel ?? "read")} · ${days}`,
    detail: "Project data only",
  };
}

interface Difference {
  role: Role;
  added: Capability[];
  removed: Capability[];
  levelDiffers: boolean;
  score: number;
}

function difference(role: Role, draft: RoleDraft): Difference {
  const theirs = holdable(role.capabilities);
  const mine = holdable(draft.capabilities);
  const added = mine.filter((capability) => !theirs.includes(capability));
  const removed = theirs.filter((capability) => !mine.includes(capability));
  const levelDiffers = role.maxLevel !== draft.maxLevel;
  return {
    role,
    added,
    removed,
    levelDiffers,
    score: added.length + removed.length + (levelDiffers ? 1 : 0),
  };
}

function describe(diff: Difference, draft: RoleDraft): string {
  if (diff.score === 0) return `Same as ${diff.role.name}`;
  const parts = [
    ...diff.added.map((capability) => `+ ${capability}`),
    ...diff.removed.map((capability) => `− ${capability}`),
    ...(diff.levelDiffers
      ? [
          `${formatLevel(draft.maxLevel)} not ${formatLevel(diff.role.maxLevel)}`,
        ]
      : []),
  ];
  const shown = parts.slice(0, 2).join(", ");
  return parts.length > 2 ? `${shown}, ${parts.length - 2} more` : shown;
}

/**
 * What a role adds up to, for the role editor's Summary: how far holders
 * reach, what they can do, their tokens, who can give the role, and the
 * nearest built-in role. Worked out from the form and the roles list.
 */
export function roleSummary(
  draft: RoleDraft,
  roles: readonly Role[],
  /** The role being viewed, which is never its own closest match. */
  self: string | null = null,
): RoleSummary {
  const mine = holdable(draft.capabilities);
  const names = mine.map((capability) => SHORT[capability] ?? capability);
  const heldList =
    names.length === 0
      ? "Nothing outside project data"
      : capitalize(
          names.length > 3
            ? `${names.slice(0, 3).join(", ")} and ${names.length - 3} more`
            : names.join(", "),
        );

  const builtIn = roles
    .filter((role) => role.builtin && role.signIn === "interactive")
    .sort((a, b) => a.rank - b.rank);
  const givers = builtIn.filter((role) => covers(role, draft));
  const giverNames = givers.map((role) => plural(role.name));

  const closest =
    builtIn
      .filter((role) => role.id !== self)
      .map((role) => difference(role, draft))
      .sort(
        (a, b) =>
          a.score - b.score ||
          a.role.rank - b.role.rank ||
          LEVELS.indexOf(a.role.maxLevel) - LEVELS.indexOf(b.role.maxLevel),
      )[0] ?? null;

  const tokens = tokenSummary(draft);
  const label = draft.name.trim() || "This role";
  return {
    maxLevel: draft.maxLevel,
    held: mine.length,
    total: ROLE_CAPABILITIES.length,
    heldList,
    tokens: tokens.value,
    tokensDetail: tokens.detail,
    givers:
      giverNames.length === 0
        ? "Only root"
        : formatList(["Root", ...giverNames]),
    giversDetail:
      giverNames.length === 0
        ? "No built-in role covers it"
        : "Anyone whose role covers it",
    closest: closest?.role ?? null,
    closestDetail: closest ? describe(closest, draft) : "",
    warnings: mine.flatMap((capability) => WARNINGS[capability] ?? []),
    sentence: `${label} reaches ${formatLevel(draft.maxLevel)} and holds ${mine.length} of ${ROLE_CAPABILITIES.length} capabilities.`,
  };
}
