import { computed, ref, watch, type Ref } from "vue";
import { createPersonalToken } from "~/services/personal-tokens.api";
import type {
  CreatedPersonalToken,
  PersonalToken,
} from "~/services/personal-tokens.api";
import type { Grant, Level } from "~/services/access.api";
import { reaches } from "~/services/access.api";
import type { Role } from "~/services/roles.api";
import type { User } from "~/services/users.api";
import { ApiError } from "~/services/http.client";
import { errorMessage } from "~/services/api-errors";
import { useReauthentication } from "~/composable/reauthentication";
import { tokenExpiresIn } from "~/utils/token-expiry";
import { isValidSlug } from "~/utils/validation";

/** Every preset; the role's longest lifetime trims the list. */
const EXPIRY_PRESETS = [
  { value: "7d", label: "7 days", days: 7 },
  { value: "30d", label: "30 days", days: 30 },
  { value: "90d", label: "90 days", days: 90 },
  { value: "365d", label: "1 year", days: 365 },
];

export const AGENT_TOKEN_LEVELS: {
  level: Level;
  label: string;
  hint: string;
}[] = [
  {
    level: "metadata",
    label: "Names only",
    hint: "List projects, environments and secret names. No values.",
  },
  { level: "read", label: "Read", hint: "Also read values, export and run." },
];

/** Where a token may reach: everything the agent does, or one place. */
export interface TokenScopeOption {
  value: string;
  label: string;
  scopeType: "all" | "project" | "environment";
  scope?: string;
}

/**
 * New token for an AI agent. An agent holds one token at a time, so the
 * new one replaces any live token in the same request. The role decides
 * how much a token may do and how long it may live; the token never
 * reaches more than the agent, nor the agent more than its owner.
 */
export function useAgentTokenDialog(
  open: Ref<boolean>,
  agent: Ref<User>,
  role: Ref<Role | null>,
  grants: Ref<Grant[]>,
  current: Ref<PersonalToken | null>,
) {
  const { runWithReauth } = useReauthentication();

  const name = ref("");
  const level = ref<Level>("metadata");
  const where = ref("all");
  const expiry = ref("30d");
  const customDays = ref("");
  const saving = ref(false);
  const nameError = ref<string | null>(null);
  const expiryError = ref<string | null>(null);
  const error = ref<string | null>(null);

  const maxDays = computed(() => role.value?.tokenMaxTtlDays ?? 90);
  const expiryOptions = computed(() => [
    ...EXPIRY_PRESETS.filter((preset) => preset.days <= maxDays.value).map(
      ({ value, label }) => ({ value, label }),
    ),
    { value: "custom", label: "Custom" },
  ]);

  const levels = computed(() =>
    AGENT_TOKEN_LEVELS.map((choice) => ({
      ...choice,
      disabled: !reaches(role.value?.tokenMaxLevel ?? "read", choice.level),
    })),
  );

  /** Its grants, as places a token can be narrowed to. */
  const scopeOptions = computed<TokenScopeOption[]>(() => {
    const options: TokenScopeOption[] = [
      {
        value: "all",
        label: `Every project ${agent.value.email} reaches`,
        scopeType: "all",
      },
    ];
    for (const grant of grants.value) {
      if (grant.level === "none") continue;
      if (grant.scopeType === "project" || grant.scopeType === "environment") {
        options.push({
          value: `${grant.scopeType}:${grant.scopeId}`,
          label:
            grant.scopeType === "project"
              ? `${grant.scopeLabel}, whole project`
              : grant.scopeLabel,
          scopeType: grant.scopeType,
          scope: grant.scopeId,
        });
      }
    }
    return options;
  });

  watch(
    open,
    (isOpen) => {
      if (!isOpen) return;
      name.value = "";
      level.value = "metadata";
      where.value = "all";
      expiry.value = maxDays.value >= 30 ? "30d" : "7d";
      customDays.value = "";
      nameError.value = null;
      expiryError.value = null;
      error.value = null;
    },
    // Immediate: a page may mount the dialog already open.
    { immediate: true },
  );

  /** The plaintext, shown once by the page. */
  const created = ref<CreatedPersonalToken | null>(null);

  async function submit(): Promise<boolean> {
    nameError.value = null;
    expiryError.value = null;
    error.value = null;
    const trimmed = name.value.trim();
    if (!trimmed) {
      nameError.value = "Name the token after where it's used, like ci-deploy.";
      return false;
    }
    if (!isValidSlug(trimmed)) {
      nameError.value =
        "Use lowercase letters, numbers and single hyphens, starting with a letter.";
      return false;
    }
    let expiresIn: string;
    try {
      expiresIn = tokenExpiresIn(expiry.value, customDays.value, "d");
    } catch (cause) {
      expiryError.value =
        cause instanceof Error ? cause.message : "Choose an expiry.";
      return false;
    }
    if (expiry.value === "custom" && Number(customDays.value) > maxDays.value) {
      expiryError.value = `Their role allows at most ${maxDays.value} days.`;
      return false;
    }
    const scope = scopeOptions.value.find(
      (option) => option.value === where.value,
    );
    saving.value = true;
    try {
      await runWithReauth(async () => {
        created.value = await createPersonalToken(agent.value.id, {
          name: trimmed,
          level: level.value,
          ...(scope && scope.scopeType !== "all"
            ? { scopeType: scope.scopeType, scope: scope.scope }
            : {}),
          expiresIn,
          replaceExisting: true,
        });
      });
      return true;
    } catch (cause) {
      if (cause instanceof DOMException && cause.name === "AbortError")
        return false;
      if (cause instanceof ApiError && cause.status === 409)
        nameError.value = "This agent already has a token with that name.";
      else error.value = errorMessage(cause, "Could not create the token.");
      return false;
    } finally {
      saving.value = false;
    }
  }

  const replaces = computed(() => current.value?.name ?? null);

  return {
    name,
    level,
    levels,
    where,
    scopeOptions,
    expiry,
    expiryOptions,
    customDays,
    maxDays,
    replaces,
    saving,
    nameError,
    expiryError,
    error,
    created,
    submit,
  };
}
