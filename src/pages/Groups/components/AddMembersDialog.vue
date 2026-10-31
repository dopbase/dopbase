<script setup lang="ts">
import { toRef } from "vue";
import {
  useAddMembersDialog,
  type MembersTarget,
} from "./AddMembersDialog.controller";
import {
  DbAlert,
  DbButton,
  DbCheckbox,
  DbModal,
  DbSearchField,
} from "~/components/ui";
import type { User } from "~/services/users.api";

/** AddMembersDialog — add one or more accounts to a group. */
const props = defineProps<{
  open: boolean;
  target: MembersTarget | null;
  people: User[];
}>();

const emit = defineEmits<{ close: []; saved: [] }>();

const dialog = useAddMembersDialog(
  toRef(props, "open"),
  toRef(props, "target"),
  toRef(props, "people"),
);
const { filter, selected, candidates, visible, saving, error, submitLabel } =
  dialog;

async function save(): Promise<void> {
  if (await dialog.submit()) emit("saved");
}
</script>

<template>
  <DbModal
    :open="open"
    :title="`Add members to ${target?.name ?? 'the group'}`"
    size="lg"
    @close="emit('close')">
    <div class="flex flex-col gap-3" data-testid="add-members-dialog">
      <p v-if="candidates.length === 0" class="text-sm text-ink-muted">
        Everyone who can join is already in this group.
      </p>
      <template v-else>
        <DbSearchField v-model="filter" placeholder="Filter by email" />
        <ul
          class="max-h-80 divide-y divide-line-soft overflow-y-auto rounded-card border border-line">
          <li
            v-for="person in visible"
            :key="person.id"
            class="flex h-11 items-center gap-3 px-3">
            <DbCheckbox
              :model-value="selected.has(person.id)"
              :label="person.email"
              @update:model-value="dialog.toggle(person.id, $event)" />
            <span class="ml-auto text-xs text-ink-muted">
              {{ person.roleName ?? person.role }}
            </span>
          </li>
          <li
            v-if="visible.length === 0"
            class="px-3 py-3 text-sm text-ink-muted">
            Nobody matches that filter.
          </li>
        </ul>
        <p class="text-sm text-ink-muted">
          They get the group's access right away, but never more than their own
          role allows.
        </p>
      </template>
      <DbAlert v-if="error">{{ error }}</DbAlert>
    </div>
    <template #footer>
      <DbButton variant="ghost" :disabled="saving" @click="emit('close')">
        Cancel
      </DbButton>
      <DbButton
        variant="primary"
        :loading="saving"
        :disabled="selected.size === 0"
        data-testid="add-members-submit"
        @click="save">
        {{ submitLabel }}
      </DbButton>
    </template>
  </DbModal>
</template>
