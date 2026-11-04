<script setup lang="ts">
import { computed, toRef } from "vue";
import { useUserDialog } from "./UserDialog.controller";
import {
  DbAlert,
  DbButton,
  DbInput,
  DbModal,
  DbRadioCard,
  DbSegment,
} from "~/components/ui";
import type { User } from "~/services/users.api";
import type { Role } from "~/services/roles.api";

/**
 * UserDialog — Add user, or Edit user when `editing` is set. Emits the
 * saved account so the page can open it or reload.
 */
const props = withDefaults(
  defineProps<{
    open: boolean;
    roles: Role[];
    ssoEnabled: boolean;
    editing?: User | null;
  }>(),
  { editing: null },
);

const emit = defineEmits<{ close: []; saved: [user: User] }>();

const dialog = useUserDialog(
  toRef(props, "open"),
  toRef(props, "editing"),
  toRef(props, "roles"),
  toRef(props, "ssoEnabled"),
);
const {
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
} = dialog;

const signInOptions = [
  { value: "password", label: "Password" },
  { value: "sso", label: "Single sign-on" },
] as const;

const title = computed(() =>
  props.editing ? `Edit ${props.editing.email}` : "Add user",
);

async function save(): Promise<void> {
  const saved = await dialog.submit();
  if (saved) emit("saved", saved);
}
</script>

<template>
  <DbModal :open="open" :title="title" size="lg" @close="emit('close')">
    <form
      id="user-dialog"
      class="flex flex-col gap-4"
      novalidate
      data-testid="user-dialog"
      @submit.prevent="save">
      <DbInput
        v-model="email"
        label="Email"
        name="email"
        type="email"
        autocomplete="off"
        mono
        :error="errors.email" />

      <div v-if="canChooseSso" class="flex flex-col gap-1.5">
        <p class="text-sm font-medium text-ink-muted">Signs in with</p>
        <DbSegment
          v-model="signIn"
          class="self-start"
          :options="signInOptions"
          label="Signs in with" />
      </div>

      <DbInput
        v-if="editing || signIn === 'password'"
        v-model="password"
        :label="editing ? 'New password' : 'Temporary password'"
        name="password"
        type="password"
        autocomplete="new-password"
        :error="errors.password"
        :hint="
          editing
            ? 'Leave empty to keep their password. Setting one signs them out everywhere.'
            : 'Share it securely. They can change it on their account page.'
        " />
      <p v-else class="text-[13px] text-ink-muted">
        They sign in through your identity provider and have no password here.
      </p>

      <div class="flex flex-col gap-1.5" role="radiogroup" aria-label="Role">
        <p class="text-sm font-medium text-ink-muted">Role</p>
        <DbRadioCard
          v-for="choice in roleChoices"
          :key="choice.id"
          name="user-role"
          :value="choice.id"
          :checked="roleId === choice.id"
          :label="choice.name"
          :description="choice.description"
          :disabled="choice.disabled"
          @select="roleId = choice.id" />
        <p v-if="errors.role" class="text-xs text-crit">{{ errors.role }}</p>
      </div>

      <DbAlert v-if="note" tone="info">{{ note }}</DbAlert>
      <DbAlert v-if="formError">{{ formError }}</DbAlert>
    </form>
    <template #footer>
      <DbButton variant="ghost" :disabled="saving" @click="emit('close')">
        Cancel
      </DbButton>
      <DbButton
        variant="primary"
        type="submit"
        form="user-dialog"
        :loading="saving"
        data-testid="user-dialog-submit">
        {{ editing ? "Save" : "Add user" }}
      </DbButton>
    </template>
  </DbModal>
</template>
