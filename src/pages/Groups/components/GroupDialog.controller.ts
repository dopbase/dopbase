import { computed, ref, watch, type Ref } from "vue";
import { createGroup, updateGroup } from "~/services/access.api";
import type { Group, GroupDetail } from "~/services/access.api";
import { errorMessage } from "~/services/api-errors";
import { useReauthentication } from "~/composable/reauthentication";

/**
 * New group, or rename one: a name and an optional description. Resets
 * each time it opens, and returns the saved group.
 */
export function useGroupDialog(open: Ref<boolean>, editing: Ref<Group | null>) {
  const { runWithReauth } = useReauthentication();

  const name = ref("");
  const description = ref("");
  const saving = ref(false);
  const nameError = ref<string | null>(null);
  const error = ref<string | null>(null);
  const saved = ref<GroupDetail | null>(null);

  const isEdit = computed(() => editing.value !== null);
  const title = computed(() =>
    editing.value ? `Rename ${editing.value.name}` : "New group",
  );

  watch(
    open,
    (isOpen) => {
      if (!isOpen) return;
      name.value = editing.value?.name ?? "";
      description.value = editing.value?.description ?? "";
      nameError.value = null;
      error.value = null;
      saved.value = null;
    },
    { immediate: true },
  );

  async function submit(): Promise<boolean> {
    const trimmed = name.value.trim();
    nameError.value = trimmed ? null : "Enter a group name.";
    if (nameError.value) return false;
    saving.value = true;
    error.value = null;
    try {
      await runWithReauth(async () => {
        saved.value = editing.value
          ? await updateGroup(editing.value.id, {
              name: trimmed,
              description: description.value.trim(),
            })
          : await createGroup(trimmed, description.value.trim());
      });
      return saved.value !== null;
    } catch (cause) {
      if (cause instanceof DOMException && cause.name === "AbortError") {
        return false;
      }
      error.value = errorMessage(
        cause,
        isEdit.value
          ? "Could not rename the group."
          : "Could not create the group.",
      );
      return false;
    } finally {
      saving.value = false;
    }
  }

  return {
    name,
    description,
    saving,
    nameError,
    error,
    saved,
    isEdit,
    title,
    submit,
  };
}
