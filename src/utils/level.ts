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
