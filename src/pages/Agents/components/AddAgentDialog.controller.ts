import { ref, watch, type Ref } from "vue";
import { createUser } from "~/services/users.api";
import type { User } from "~/services/users.api";
import type { Role } from "~/services/roles.api";
import { ApiError } from "~/services/http.client";
import { errorMessage } from "~/services/api-errors";
import { useReauthentication } from "~/composable/reauthentication";

/**
 * Add AI agent: just a name. You own the agent, it never reaches more than
 * you do, and it starts with no access until someone gives it some.
 */
export function useAddAgentDialog(open: Ref<boolean>, role: Ref<Role | null>) {
  const { runWithReauth } = useReauthentication();
  const name = ref("");
  const saving = ref(false);
  const nameError = ref<string | null>(null);
  const error = ref<string | null>(null);

  watch(
    open,
    (isOpen) => {
      if (!isOpen) return;
      name.value = "";
      nameError.value = null;
      error.value = null;
    },
    // Immediate: a page may mount the dialog already open.
    { immediate: true },
  );

  async function submit(): Promise<User | null> {
    nameError.value = null;
    error.value = null;
    const trimmed = name.value.trim();
    if (!trimmed) {
      nameError.value = "Name the agent after what it does, like review-bot.";
      return null;
    }
    if (!role.value) {
      error.value = "There's no AI agent role you can give.";
      return null;
    }
    saving.value = true;
    let created: User | null = null;
    try {
      await runWithReauth(async () => {
        created = await createUser(trimmed, "", { roleId: role.value!.id });
      });
      return created;
    } catch (cause) {
      if (cause instanceof DOMException && cause.name === "AbortError")
        return null;
      if (
        cause instanceof ApiError &&
        (cause.hasCode("EMAIL_TAKEN") || cause.hasCode("EMAIL_INVALID"))
      )
        nameError.value =
          cause.codes.EMAIL_TAKEN ?? cause.codes.EMAIL_INVALID ?? cause.message;
      else error.value = errorMessage(cause, "Could not add the agent.");
      return null;
    } finally {
      saving.value = false;
    }
  }

  return { name, saving, nameError, error, submit };
}
