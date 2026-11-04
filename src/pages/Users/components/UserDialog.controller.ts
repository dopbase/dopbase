import { computed, ref, watch, type Ref } from "vue";
import { ApiError } from "~/services/http.client";
import { createUser, updateUser } from "~/services/users.api";
import type { User } from "~/services/users.api";
import type { Role } from "~/services/roles.api";
import { useReauthentication } from "~/composable/reauthentication";
import {
  isValidEmail,
  MAX_PASSWORD_LENGTH,
  MIN_PASSWORD_LENGTH,
  validatePasswordLength,
} from "~/utils/validation";

export type SignInChoice = "password" | "sso";

export interface RoleChoice {
  id: string;
  name: string;
  description: string;
  /** Your own role can't give this one. */
  disabled: boolean;
}

/**
 * Add user and Edit user dialog. Adding takes an email, how they sign in
 * (a temporary password, or single sign-on only while it's on) and a role.
 * Editing changes the email, role, or sets a new password.
 *
 * Only roles people sign in with are offered; AI agents are added on their
 * own page. Roles beyond your own show but can't be picked.
 */
export function useUserDialog(
  open: Ref<boolean>,
  editing: Ref<User | null>,
  roles: Ref<Role[]>,
  ssoEnabled: Ref<boolean>,
) {
  const { runWithReauth } = useReauthentication();

  const email = ref("");
  const signIn = ref<SignInChoice>("password");
  const password = ref("");
  const roleId = ref("");
  const saving = ref(false);
  const formError = ref<string | null>(null);
  const errors = ref<{ email?: string; password?: string; role?: string }>({});

  const roleChoices = computed<RoleChoice[]>(() =>
    roles.value
      .filter((role) => role.signIn === "interactive")
      .map((role) => ({
        id: role.id,
        name: role.name,
        description: role.assignable
          ? role.description
          : `${role.description ? `${role.description} ` : ""}Your role can't give this one.`.trim(),
        disabled: !role.assignable && role.id !== editing.value?.roleId,
      })),
  );

  watch(
    open,
    (isOpen) => {
      if (!isOpen) return;
      formError.value = null;
      errors.value = {};
      password.value = "";
      signIn.value = "password";
      const user = editing.value;
      email.value = user?.email ?? "";
      roleId.value =
        user?.roleId ??
        roleChoices.value.find(
          (choice) => choice.name === "Member" && !choice.disabled,
        )?.id ??
        roleChoices.value.find((choice) => !choice.disabled)?.id ??
        "";
    },
    // Immediate: a page may mount the dialog already open.
    { immediate: true },
  );

  function passwordProblem(value: string): string | undefined {
    const check = validatePasswordLength(value);
    if (check.code === "PASSWORD_TOO_SHORT")
      return `Use at least ${MIN_PASSWORD_LENGTH} characters.`;
    if (check.code === "PASSWORD_TOO_LONG")
      return `Use at most ${MAX_PASSWORD_LENGTH} characters.`;
    return undefined;
  }

  function validate(): boolean {
    const next: typeof errors.value = {};
    const trimmed = email.value.trim();
    if (!trimmed) next.email = "Enter their email.";
    else if (!isValidEmail(trimmed))
      next.email = "That doesn't look like an email address.";
    const needsPassword = !editing.value && signIn.value === "password";
    if (needsPassword && !password.value)
      next.password = "Set a temporary password.";
    else if (password.value) {
      const problem = passwordProblem(password.value);
      if (problem) next.password = problem;
    }
    if (!roleId.value) next.role = "Pick a role.";
    errors.value = next;
    return Object.keys(next).length === 0;
  }

  /** The saved account, or null when it wasn't saved. */
  async function submit(): Promise<User | null> {
    formError.value = null;
    if (!validate()) return null;
    saving.value = true;
    let saved: User | null = null;
    try {
      await runWithReauth(async () => {
        const trimmed = email.value.trim();
        const user = editing.value;
        if (user) {
          saved = await updateUser(user.id, {
            ...(trimmed !== user.email ? { email: trimmed } : {}),
            ...(roleId.value !== user.roleId ? { roleId: roleId.value } : {}),
            ...(password.value ? { password: password.value } : {}),
          });
        } else {
          saved = await createUser(
            trimmed,
            signIn.value === "password" ? password.value : "",
            { roleId: roleId.value },
          );
        }
      });
      return saved;
    } catch (cause) {
      if (cause instanceof DOMException && cause.name === "AbortError")
        return null;
      if (cause instanceof ApiError) {
        const codes = cause.codes;
        if (codes.EMAIL_INVALID || codes.EMAIL_TAKEN)
          errors.value = {
            ...errors.value,
            email: codes.EMAIL_INVALID ?? codes.EMAIL_TAKEN,
          };
        else if (codes.PASSWORD_TOO_SHORT || codes.PASSWORD_TOO_LONG)
          errors.value = {
            ...errors.value,
            password: codes.PASSWORD_TOO_SHORT ?? codes.PASSWORD_TOO_LONG,
          };
        else formError.value = cause.message;
      } else {
        formError.value = "The account could not be saved.";
      }
      return null;
    } finally {
      saving.value = false;
    }
  }

  /** "sam@example.com reaches no project until someone gives them access." */
  const note = computed(() => {
    if (editing.value) {
      return roleId.value !== editing.value.roleId
        ? "Changing the role signs them out everywhere and resets any access to all projects that came with the old role."
        : "";
    }
    const who = email.value.trim();
    const reaches = who ? `${who} reaches` : "They reach";
    return `${reaches} no project until someone gives them access. You can do that from their page right after this.`;
  });

  const canChooseSso = computed(() => !editing.value && ssoEnabled.value);

  return {
    email,
    signIn,
    password,
    roleId,
    roleChoices,
    canChooseSso,
    note,
    saving,
    formError,
    errors,
    submit,
  };
}
