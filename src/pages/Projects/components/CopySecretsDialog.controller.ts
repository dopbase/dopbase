import { computed, ref, watch } from "vue";
import type { Ref } from "vue";
import { useQuery } from "@pinia/colada";
import * as secretsApi from "~/services/secrets.api";
import type {
  CopySecretsResponse,
  SecretMetadata,
} from "~/services/secrets.api";
import { ApiError } from "~/services/http.client";
import { loadFailure, onlyWhen } from "~/queries/helpers";
import { secretsQuery } from "~/queries/secrets";
import type { CopySource } from "./EmptyEnvironment.controller";

/**
 * Copy secrets dialog controller: lists the source's keys when the dialog
 * opens, all ticked, and copies the ticked ones into the target in one
 * request. Values are copied only when asked for and only on the server.
 *
 * The keys come from the source's cached secret list, which its own
 * Secrets tab reads too.
 */
export function useCopySecretsDialogController(
  targetId: Ref<string>,
  source: Ref<CopySource | null>,
  open: Ref<boolean>,
  initialValues: Ref<boolean>,
) {
  const sourceKeys = useQuery(() =>
    onlyWhen(
      open.value && source.value !== null,
      secretsQuery(source.value?.id ?? ""),
    ),
  );
  const keys = computed<SecretMetadata[] | null>(() => {
    const list = sourceKeys.data.value;
    return list ? [...list].sort((a, b) => a.key.localeCompare(b.key)) : null;
  });
  const loadError = loadFailure(
    sourceKeys,
    () => `Could not load the keys in ${source.value?.name ?? "it"}.`,
  );
  const selected = ref(new Set<string>());
  const values = ref(false);
  const submitting = ref(false);
  const submitError = ref<string | null>(null);

  /** Tries again after a failed load. */
  async function load(): Promise<void> {
    if (source.value) await sourceKeys.refetch();
  }

  watch(
    open,
    (isOpen) => {
      if (!isOpen) return;
      values.value = initialValues.value && !!source.value?.canCopyValues;
      submitError.value = null;
    },
    { immediate: true },
  );

  // Everything starts ticked, once per opening, so a background refresh
  // never undoes what was unticked.
  let tickedFor: string | null = null;
  watch(
    [keys, open, () => source.value?.id ?? null],
    ([list, isOpen, sourceId]) => {
      if (!isOpen) {
        tickedFor = null;
        return;
      }
      if (!list || tickedFor === sourceId) return;
      tickedFor = sourceId;
      selected.value = new Set(list.map((secret) => secret.key));
    },
    { immediate: true },
  );

  const selectedCount = computed(() => selected.value.size);
  const allSelected = computed(
    () =>
      keys.value !== null &&
      keys.value.length > 0 &&
      selected.value.size === keys.value.length,
  );
  const someSelected = computed(
    () => selected.value.size > 0 && !allSelected.value,
  );

  function toggle(key: string, on: boolean): void {
    const next = new Set(selected.value);
    if (on) next.add(key);
    else next.delete(key);
    selected.value = next;
  }

  function toggleAll(on: boolean): void {
    selected.value = new Set(
      on ? (keys.value ?? []).map((secret) => secret.key) : [],
    );
  }

  /** Copies the ticked keys. Resolves null when it failed. */
  async function submit(): Promise<CopySecretsResponse | null> {
    const from = source.value;
    if (!from || selected.value.size === 0 || submitting.value) return null;
    submitting.value = true;
    submitError.value = null;
    try {
      return await secretsApi.copySecrets(
        targetId.value,
        from.id,
        [...selected.value],
        values.value,
      );
    } catch (error) {
      submitError.value =
        error instanceof ApiError && error.status !== 0 && error.status < 500
          ? error.message
          : "The keys could not be copied. Try again.";
      return null;
    } finally {
      submitting.value = false;
    }
  }

  return {
    keys,
    loadError,
    selected,
    selectedCount,
    allSelected,
    someSelected,
    values,
    submitting,
    submitError,
    load,
    toggle,
    toggleAll,
    submit,
  };
}
