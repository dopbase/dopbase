import { computed, ref, watch, type Ref } from "vue";
import { setGrant } from "~/services/access.api";
import type { Grant, Level } from "~/services/access.api";
import { errorMessage } from "~/services/api-errors";
import { useReauthentication } from "~/composable/reauthentication";
import { formatLevel } from "~/utils/format";
import {
  grantSentence,
  levelChoices,
  type GrantPrincipal,
} from "~/utils/level";

/**
 * Change access dialog: picks a new level for a grant someone already
 * holds. The grant's place is fixed; giving new access has its own page.
 * Levels above the principal's role are shown but can't be picked.
 *
 * The server checks it all again: you need Project manager on the project,
 * and nobody can be given more than their role allows.
 */
export function useChangeAccessDialog(
  principal: Ref<GrantPrincipal>,
  grant: Ref<Grant | null>,
) {
  const { runWithReauth } = useReauthentication();

  const level = ref<Level>("read");
  const saving = ref(false);
  const error = ref<string | null>(null);

  watch(
    grant,
    (current) => {
      if (!current) return;
      level.value = current.level;
      error.value = null;
      saving.value = false;
    },
    // Immediate: a page may mount the dialog already open.
    { immediate: true },
  );

  const levels = computed(() =>
    levelChoices(
      grant.value?.scopeType === "environment" ? "environment" : "project",
      principal.value.maxLevel,
    ),
  );

  const target = computed(() => {
    const current = grant.value;
    if (!current) return "";
    return current.scopeType === "environment"
      ? current.scopeLabel
      : `all of ${current.scopeLabel}`;
  });

  const summary = computed(() =>
    grantSentence(
      principal.value.label,
      principal.value.type === "group",
      level.value,
      target.value,
    ),
  );

  async function submit(): Promise<boolean> {
    const current = grant.value;
    if (!current) return false;
    error.value = null;
    if (levels.value.find((choice) => choice.level === level.value)?.disabled) {
      error.value = `Their role allows up to ${formatLevel(principal.value.maxLevel ?? "none")}.`;
      return false;
    }
    saving.value = true;
    try {
      await runWithReauth(async () => {
        await setGrant({
          principalType: principal.value.type,
          principalId: principal.value.id,
          scopeType: current.scopeType,
          scope: current.scopeId,
          level: level.value,
        });
      });
      return true;
    } catch (cause) {
      error.value = errorMessage(cause, "Could not change that access.");
      return false;
    } finally {
      saving.value = false;
    }
  }

  return { level, levels, summary, saving, error, submit };
}
