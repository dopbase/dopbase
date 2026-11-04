import { computed, ref, watch, type Ref } from "vue";
import { createPersonalToken } from "~/services/personal-tokens.api";
import type {
  CreatedPersonalToken,
  CreatePersonalTokenRequest,
} from "~/services/personal-tokens.api";
import type { Environment } from "~/services/environments.api";
import type { Project } from "~/services/projects.api";
import type { Level } from "~/services/access.api";
import { reaches } from "~/services/access.api";
import { ApiError } from "~/services/http.client";
import { errorMessage } from "~/services/api-errors";
import { useReauthentication } from "~/composable/reauthentication";
import { expiryDate, tokenExpiresIn } from "~/utils/token-expiry";
import { formatDate } from "~/utils/format";
import { isValidSlug } from "~/utils/validation";

/** Lifetimes on offer; the role's longest one trims the list. */
const EXPIRY_PRESETS = [
  { value: "7d", label: "7 days", days: 7 },
  { value: "30d", label: "30 days", days: 30 },
  { value: "90d", label: "90 days", days: 90 },
];

export type TokenPreset = "agent" | "script" | "custom";

/** "Start from": the common cases fill in a safe level and lifetime. */
export const TOKEN_PRESETS: {
  value: TokenPreset;
  label: string;
  level?: Level;
  expiry?: string;
}[] = [
  { value: "agent", label: "AI agent", level: "metadata", expiry: "30d" },
  { value: "script", label: "Script", level: "read", expiry: "90d" },
  { value: "custom", label: "Custom" },
];

export const TOKEN_LEVELS: { level: Level; label: string; hint: string }[] = [
  {
    level: "metadata",
    label: "Names only",
    hint: "Enough for checking config and writing .env.example. No values.",
  },
  { level: "read", label: "Read", hint: "Also read values, export and run." },
  { level: "write", label: "Write", hint: "Also change secrets." },
];

/** What a token at each level does, for the summary line. */
const LEVEL_VERBS: Partial<Record<Level, string>> = {
  metadata: "list names in",
  read: "read secrets in",
  write: "read and change secrets in",
};

export interface TokenScopeChoice {
  value: string;
  label: string;
  scopeType: "all" | "project" | "environment";
  scope?: string;
}

/** The token's limits, from your role. */
export interface TokenLimits {
  /** The deepest level your tokens may hold. */
  level: Level;
  /** The longest lifetime, in days. */
  days: number;
}

/**
 * New personal token, or New audit token for roles whose tokens read the
 * audit log. A data token picks a level no higher than the role allows,
 * every project you reach or one place, and a lifetime within the role's
 * maximum. It never reaches more than you do.
 */
export function usePersonalTokenDialog(
  open: Ref<boolean>,
  auditOnly: Ref<boolean>,
  projects: Ref<Project[]>,
  environments: Ref<Environment[]>,
  limits: Ref<TokenLimits>,
) {
  const { runWithReauth } = useReauthentication();

  const preset = ref<TokenPreset>("agent");
  const name = ref("");
  const level = ref<Level>("metadata");
  const where = ref("all");
  const expiry = ref("30d");
  const customDays = ref("");
  const saving = ref(false);
  const nameError = ref<string | null>(null);
  const expiryError = ref<string | null>(null);
  const error = ref<string | null>(null);
  const created = ref<CreatedPersonalToken | null>(null);

  const maxDays = computed(() => limits.value.days);

  const levels = computed(() =>
    TOKEN_LEVELS.map((choice) => ({
      ...choice,
      disabled: !reaches(limits.value.level, choice.level),
    })),
  );

  const expiryOptions = computed(() => [
    ...EXPIRY_PRESETS.filter((option) => option.days <= maxDays.value).map(
      ({ value, label }) => ({ value, label }),
    ),
    { value: "custom", label: "Custom" },
  ]);

  const scopeOptions = computed<TokenScopeChoice[]>(() => {
    const options: TokenScopeChoice[] = [
      { value: "all", label: "Every project you reach", scopeType: "all" },
    ];
    for (const project of projects.value) {
      options.push({
        value: `project:${project.id}`,
        label: project.name,
        scopeType: "project",
        scope: project.id,
      });
      for (const environment of environments.value) {
        if (environment.projectId !== project.id) continue;
        options.push({
          value: `environment:${environment.id}`,
          label: `${project.name}/${environment.name}`,
          scopeType: "environment",
          scope: environment.id,
        });
      }
    }
    return options;
  });

  /** The level and lifetime the last preset set, after the role's limits. */
  let applied: { level: Level; expiry: string } | null = null;

  /** Highest allowed level at or below `wanted`. */
  function allowedLevel(wanted: Level): Level {
    const allowed = levels.value.filter((choice) => !choice.disabled);
    const fit = [...allowed]
      .reverse()
      .find((choice) => reaches(wanted, choice.level));
    return fit?.level ?? allowed[0]?.level ?? "metadata";
  }

  /** Longest offered lifetime at or below `wanted`. */
  function allowedExpiry(wanted: string): string {
    const options = expiryOptions.value.filter(
      (option) => option.value !== "custom",
    );
    if (options.some((option) => option.value === wanted)) return wanted;
    return options.at(-1)?.value ?? "custom";
  }

  function choosePreset(value: TokenPreset): void {
    preset.value = value;
    const chosen = TOKEN_PRESETS.find((option) => option.value === value);
    if (!chosen?.level || !chosen.expiry) {
      applied = null;
      return;
    }
    level.value = allowedLevel(chosen.level);
    expiry.value = allowedExpiry(chosen.expiry);
    applied = { level: level.value, expiry: expiry.value };
  }

  // Changing what a preset set makes it a custom token.
  watch([level, expiry], () => {
    if (
      applied &&
      (level.value !== applied.level || expiry.value !== applied.expiry)
    ) {
      preset.value = "custom";
      applied = null;
    }
  });

  watch(
    open,
    (isOpen) => {
      if (!isOpen) return;
      name.value = "";
      where.value = "all";
      customDays.value = "";
      nameError.value = null;
      expiryError.value = null;
      error.value = null;
      created.value = null;
      choosePreset("agent");
    },
    // Immediate: a page may mount the dialog already open.
    { immediate: true },
  );

  /** `30d`, or null while a custom lifetime isn't a valid number yet. */
  const expiresIn = computed(() => {
    try {
      return tokenExpiresIn(expiry.value, customDays.value, "d");
    } catch {
      return null;
    }
  });

  /** One sentence on what the token will be able to do, and until when. */
  const summary = computed(() => {
    const who = name.value.trim() || "This token";
    const until = expiresIn.value ? expiryDate(expiresIn.value) : null;
    const when = until ? ` until ${formatDate(until.toISOString())}` : "";
    if (auditOnly.value) {
      return `${who} can read the audit log you see${when}. It never reaches projects or secrets.`;
    }
    const place =
      scopeOptions.value.find((option) => option.value === where.value) ??
      scopeOptions.value[0];
    const placeLabel =
      place.scopeType === "all" ? "every project you reach" : place.label;
    return `${who} can ${LEVEL_VERBS[level.value] ?? "reach"} ${placeLabel}${when}, and never more than you can.`;
  });

  function request(
    trimmed: string,
    lifetime: string,
  ): CreatePersonalTokenRequest {
    if (auditOnly.value)
      return { name: trimmed, scopeType: "audit", expiresIn: lifetime };
    const place = scopeOptions.value.find(
      (option) => option.value === where.value,
    );
    return {
      name: trimmed,
      level: level.value,
      expiresIn: lifetime,
      ...(place && place.scopeType !== "all"
        ? { scopeType: place.scopeType, scope: place.scope }
        : { scopeType: "all" as const }),
    };
  }

  async function submit(): Promise<boolean> {
    nameError.value = null;
    expiryError.value = null;
    error.value = null;
    const trimmed = name.value.trim();
    if (!trimmed) {
      nameError.value = auditOnly.value
        ? "Name it after what reads the log, like splunk-forwarder."
        : "Name it after where it's used, like claude-laptop.";
      return false;
    }
    if (!isValidSlug(trimmed)) {
      nameError.value =
        "Use lowercase letters, numbers and single hyphens, starting with a letter.";
      return false;
    }
    let lifetime: string;
    try {
      lifetime = tokenExpiresIn(expiry.value, customDays.value, "d");
    } catch (cause) {
      expiryError.value =
        cause instanceof Error ? cause.message : "Choose an expiry.";
      return false;
    }
    if (expiry.value === "custom" && Number(customDays.value) > maxDays.value) {
      expiryError.value = `Your role allows at most ${maxDays.value} days.`;
      return false;
    }
    saving.value = true;
    try {
      await runWithReauth(async () => {
        created.value = await createPersonalToken(
          "me",
          request(trimmed, lifetime),
        );
      });
      return true;
    } catch (cause) {
      if (cause instanceof DOMException && cause.name === "AbortError")
        return false;
      if (cause instanceof ApiError && cause.status === 409)
        nameError.value = "You already have a token with that name.";
      else error.value = errorMessage(cause, "Could not create the token.");
      return false;
    } finally {
      saving.value = false;
    }
  }

  return {
    preset,
    presets: TOKEN_PRESETS.map(({ value, label }) => ({ value, label })),
    choosePreset,
    name,
    level,
    levels,
    where,
    scopeOptions,
    expiry,
    expiryOptions,
    customDays,
    maxDays,
    summary,
    saving,
    nameError,
    expiryError,
    error,
    created,
    submit,
  };
}
