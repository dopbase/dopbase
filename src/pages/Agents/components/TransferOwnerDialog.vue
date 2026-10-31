<script setup lang="ts">
import { toRef } from "vue";
import { useTransferOwnerDialog } from "./TransferOwnerDialog.controller";
import { DbAlert, DbButton, DbModal, DbSelect } from "~/components/ui";
import type { User } from "~/services/users.api";

/** TransferOwnerDialog — picks the person an AI agent answers to. */
const props = defineProps<{
  open: boolean;
  agent: User | null;
  people: User[];
}>();
const emit = defineEmits<{ close: []; saved: [] }>();

const dialog = useTransferOwnerDialog(
  toRef(props, "open"),
  toRef(props, "agent"),
  toRef(props, "people"),
);
const { ownerId, options, saving, error } = dialog;

async function save(): Promise<void> {
  if (await dialog.submit()) emit("saved");
}
</script>

<template>
  <DbModal
    :open="open"
    :title="`Transfer ${agent?.email ?? 'agent'}`"
    @close="emit('close')">
    <form
      id="transfer-owner"
      class="flex flex-col gap-4"
      novalidate
      data-testid="transfer-owner-dialog"
      @submit.prevent="save">
      <DbSelect
        v-model="ownerId"
        label="New owner"
        :options="options"
        :disabled="options.length === 0" />
      <DbAlert tone="info">
        The agent keeps its grants and token, but never reaches more than its
        new owner does.
      </DbAlert>
      <DbAlert v-if="error">{{ error }}</DbAlert>
    </form>
    <template #footer>
      <DbButton variant="ghost" :disabled="saving" @click="emit('close')">
        Cancel
      </DbButton>
      <DbButton
        variant="primary"
        type="submit"
        form="transfer-owner"
        :loading="saving"
        :disabled="options.length === 0">
        Transfer
      </DbButton>
    </template>
  </DbModal>
</template>
