<script setup lang="ts">
import { toRef } from "vue";
import { useAddAgentDialog } from "./AddAgentDialog.controller";
import { DbAlert, DbButton, DbInput, DbModal } from "~/components/ui";
import type { Role } from "~/services/roles.api";
import type { User } from "~/services/users.api";

/** AddAgentDialog — names a new AI agent; the page opens it after. */
const props = defineProps<{ open: boolean; role: Role | null }>();
const emit = defineEmits<{ close: []; added: [agent: User] }>();

const dialog = useAddAgentDialog(toRef(props, "open"), toRef(props, "role"));
const { name, saving, nameError, error } = dialog;

async function save(): Promise<void> {
  const agent = await dialog.submit();
  if (agent) emit("added", agent);
}
</script>

<template>
  <DbModal :open="open" title="Add AI agent" @close="emit('close')">
    <form
      id="add-agent"
      class="flex flex-col gap-4"
      novalidate
      data-testid="add-agent-dialog"
      @submit.prevent="save">
      <DbInput
        v-model="name"
        label="Name"
        name="agentName"
        placeholder="review-bot"
        mono
        :error="nameError" />
      <DbAlert tone="info">
        You own this agent, and it never reaches more than you do. It starts
        with no access: give it some from its page, then create its token.
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
        form="add-agent"
        :loading="saving"
        data-testid="add-agent-submit">
        Add agent
      </DbButton>
    </template>
  </DbModal>
</template>
