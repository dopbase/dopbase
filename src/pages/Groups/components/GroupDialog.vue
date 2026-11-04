<script setup lang="ts">
import { toRef } from "vue";
import { useGroupDialog } from "./GroupDialog.controller";
import { DbAlert, DbButton, DbInput, DbModal } from "~/components/ui";
import type { Group, GroupDetail } from "~/services/access.api";

/** GroupDialog — New group, or rename one. Emits the saved group. */
const props = withDefaults(
  defineProps<{ open: boolean; editing?: Group | null }>(),
  { editing: null },
);

const emit = defineEmits<{ close: []; saved: [group: GroupDetail] }>();

const dialog = useGroupDialog(toRef(props, "open"), toRef(props, "editing"));
const { name, description, saving, nameError, error, saved, isEdit, title } =
  dialog;

async function save(): Promise<void> {
  if ((await dialog.submit()) && saved.value) emit("saved", saved.value);
}
</script>

<template>
  <DbModal :open="open" :title="title" @close="emit('close')">
    <form
      id="group-dialog"
      class="flex flex-col gap-4"
      novalidate
      data-testid="group-dialog"
      @submit.prevent="save">
      <DbInput
        v-model="name"
        label="Name"
        name="groupName"
        placeholder="Backend"
        :error="nameError" />
      <DbInput
        v-model="description"
        label="Description (optional)"
        name="groupDescription"
        placeholder="Payments service team" />
      <p v-if="!isEdit" class="text-sm text-ink-muted">
        The group reaches nothing until you give it access. Everyone you add
        then gets that access, capped by their own role.
      </p>
      <DbAlert v-if="error">{{ error }}</DbAlert>
    </form>
    <template #footer>
      <DbButton variant="ghost" :disabled="saving" @click="emit('close')">
        Cancel
      </DbButton>
      <DbButton
        variant="primary"
        type="submit"
        form="group-dialog"
        :loading="saving"
        data-testid="group-dialog-submit">
        {{ isEdit ? "Save" : "Create group" }}
      </DbButton>
    </template>
  </DbModal>
</template>
