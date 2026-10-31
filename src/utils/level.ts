import { LEVELS } from "~/services/access.api";
import type { Level } from "~/services/access.api";
import { formatLevel } from "./format";

/** What `DbLevelBadge` draws for an access level. */
export interface LevelMeter {
  label: string;
  /** Bars lit out of five: Names only is one, Project manager is five. */
  filled: number;
  /** Manage and Project manager light their bars in the accent colour. */
  strong: boolean;
}

export function levelMeter(level: Level): LevelMeter {
  return {
    label: formatLevel(level),
    filled: LEVELS.indexOf(level),
    strong: LEVELS.indexOf(level) >= LEVELS.indexOf("manage"),
  };
}

/** Levels a grant can give, by what it covers. */
export const PROJECT_LEVELS: Level[] = [
  "metadata",
  "read",
  "write",
  "manage",
  "project_manager",
];
export const ENVIRONMENT_LEVELS: Level[] = [
  "none",
  "metadata",
  "read",
  "write",
  "manage",
];

export const LEVEL_HINTS: Record<Level, string> = {
  none: "Hides this environment, even from a project grant.",
  metadata: "See secret names, versions and history. No values.",
  read: "Also reveal, export and run with values.",
  write: "Also add, change, import and roll back secrets.",
  manage: "Also runner tokens, renaming and protecting environments.",
  project_manager:
    "Everything here, including giving others access. Whole project only.",
};

/** What a level lets someone do, for the summary under a grant dialog. */
export const LEVEL_VERBS: Record<Level, string> = {
  none: "no longer reach",
  metadata: "see the secret names in",
  read: "read secrets in",
  write: "read and change secrets in",
  manage: "manage",
  project_manager: "manage, and give others access to,",
};

/**
 * One sentence on what a grant will allow: "maya@example.com will be able
 * to read secrets in payments / staging." `who` is a person's or agent's
 * name, or a group's, with `isGroup` set.
 */
export function grantSentence(
  who: string,
  isGroup: boolean,
  level: Level,
  target: string,
): string {
  const subject = isGroup ? `Everyone in ${who}` : who;
  return level === "none"
    ? `${subject} will no longer reach ${target}, even through a project grant.`
    : `${subject} will be able to ${LEVEL_VERBS[level]} ${target}.`;
}
