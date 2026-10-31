import { computed, ref } from "vue";
import {
  deleteUser,
  disableUser,
  enableUser,
  unlinkUserSso,
} from "~/services/users.api";
import type { User } from "~/services/users.api";
import { ApiError } from "~/services/http.client";
import { errorMessage } from "~/services/api-errors";
import { useReauthentication } from "~/composable/reauthentication";

export type AccountActionKind = "disable" | "enable" | "unlink" | "delete";

export interface AccountAction {
  kind: AccountActionKind;
  user: User;
}

/** What each confirmation says, so the dialogs read the same everywhere. */
function describe(action: AccountAction) {
  const name = action.user.email;
  switch (action.kind) {
    case "disable":
      return {
        title: `Disable ${name}?`,
        description:
          "They're signed out everywhere and can't sign in, and their tokens stop working. Their access and history stay, so you can turn the account back on.",
        confirmLabel: "Disable account",
        tone: "danger" as const,
        confirmWord: undefined,
      };
    case "enable":
      return {
        title: `Turn ${name} back on?`,
        description:
          "They can sign in again, and tokens that haven't expired work again, with the access they had before.",
        confirmLabel: "Turn back on",
        tone: "primary" as const,
        confirmWord: undefined,
      };
    case "unlink":
      return {
        title: `Unlink single sign-on for ${name}?`,
        description: action.user.hasPassword
          ? "They sign in with their password until they link single sign-on again."
          : "They have no password, so they can't sign in until single sign-on links again on their next sign-in, or you set them a password.",
        confirmLabel: "Unlink",
        tone: "danger" as const,
        confirmWord: undefined,
      };
    case "delete":
      return {
        title: `Delete ${name}?`,
        description:
          "This can't be undone. Their grants, group memberships and tokens go with the account. Audit events stay.",
        confirmLabel: "Delete account",
        tone: "danger" as const,
        confirmWord: name,
      };
  }
}

/**
 * Disable, turn back on, unlink single sign-on and delete, each behind a
 * confirmation. Shared by the users list, a user's page and AI agents, so
 * the wording and the checks are the same wherever they start.
 */
export function useAccountActions(
  done: (action: AccountAction) => void | Promise<void>,
) {
  const { runWithReauth } = useReauthentication();
  const pending = ref<AccountAction | null>(null);
  const busy = ref(false);
  const error = ref<string | null>(null);

  const copy = computed(() => (pending.value ? describe(pending.value) : null));

  function ask(kind: AccountActionKind, user: User): void {
    error.value = null;
    pending.value = { kind, user };
  }

  function cancel(): void {
    if (!busy.value) pending.value = null;
  }

  async function confirm(): Promise<void> {
    const action = pending.value;
    if (!action) return;
    busy.value = true;
    error.value = null;
    try {
      await runWithReauth(async () => {
        const id = action.user.id;
        if (action.kind === "disable") await disableUser(id);
        else if (action.kind === "enable") await enableUser(id);
        else if (action.kind === "unlink") await unlinkUserSso(id);
        else await deleteUser(id);
      });
      pending.value = null;
      await done(action);
    } catch (cause) {
      if (cause instanceof DOMException && cause.name === "AbortError") return;
      error.value =
        cause instanceof ApiError && cause.hasCode("OWNS_AI_AGENTS")
          ? "They own AI agents. Move those agents to someone else on the AI agents page first."
          : errorMessage(cause, "That didn't work. Try again.");
    } finally {
      busy.value = false;
    }
  }

  return { pending, copy, busy, error, ask, cancel, confirm };
}
