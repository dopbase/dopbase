<script setup lang="ts">
import { toRef } from "vue";
import { useTransferOwnerDialog } from "./TransferOwnerDialog.controller";
import { DbAlert, DbButton, DbModal, DbSelect } from "~/components/ui";
import type { User } from "~/services/users.api";
import type { Role } from "~/services/roles.api";

/** TransferOwnerDialog — picks the person an AI agent answers to. */
const props = defineProps<{
  open: boolean;
  agent: User | null;
  people: User[];
  roles: Role[];
}>();
const emit = defineEmits<{ close: []; saved: [] }>();

const dialog = useTransferOwnerDialog(
  toRef(props, "open"),
  toRef(props, "agent"),
  toRef(props, "people"),
  toRef(props, "roles"),
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
        Only people who can create AI agents can own one. It keeps its grants
        but never reaches more than its new owner does. Its tokens are revoked,
        so the new owner needs to create a new one.
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
