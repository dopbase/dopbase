<script setup lang="ts">
import { toRef } from "vue";
import { useAgentTokenDialog } from "./AgentTokenDialog.controller";
import {
  DbAlert,
  DbButton,
  DbInput,
  DbModal,
  DbRadioCard,
  DbSegment,
  DbSelect,
} from "~/components/ui";
import type {
  CreatedPersonalToken,
  PersonalToken,
} from "~/services/personal-tokens.api";
import type { Grant } from "~/services/access.api";
import type { Role } from "~/services/roles.api";
import type { User } from "~/services/users.api";

/**
 * AgentTokenDialog — New token for an AI agent. Warns that it replaces the
 * live token, and emits the created token so the page can show it once.
 */
const props = defineProps<{
  open: boolean;
  agent: User;
  role: Role | null;
  grants: Grant[];
  /** The agent's live token, which the new one replaces. */
  current: PersonalToken | null;
}>();

const emit = defineEmits<{
  close: [];
  created: [token: CreatedPersonalToken];
}>();

const dialog = useAgentTokenDialog(
  toRef(props, "open"),
  toRef(props, "agent"),
  toRef(props, "role"),
  toRef(props, "grants"),
  toRef(props, "current"),
);
const {
  name,
  level,
  levels,
  where,
  scopeOptions,
  expiry,
  expiryOptions,
  customDays,
  maxDays,
  replaces,
  saving,
  nameError,
  expiryError,
  error,
  created,
} = dialog;

async function save(): Promise<void> {
  if ((await dialog.submit()) && created.value) emit("created", created.value);
}
</script>

<template>
  <DbModal
    :open="open"
    :title="`New token for ${agent.email}`"
    size="lg"
    @close="emit('close')">
    <form
      id="agent-token"
      class="flex flex-col gap-4"
      novalidate
      data-testid="agent-token-dialog"
      @submit.prevent="save">
      <DbAlert
        v-if="replaces"
        tone="info"
        title="This replaces the current token">
        An agent has one token at a time. Creating this one revokes
        <span class="font-mono">{{ replaces }}</span> right away, so update
        wherever it's used.
      </DbAlert>
      <DbInput
        v-model="name"
        label="Name"
        name="tokenName"
        placeholder="ci-deploy"
        mono
        :error="nameError"
        hint="Name it after where it's used." />
      <div class="flex flex-col gap-1.5" role="radiogroup" aria-label="Can">
        <p class="text-sm font-medium text-ink-muted">Can</p>
        <DbRadioCard
          v-for="choice in levels"
          :key="choice.level"
          name="agent-token-level"
          :value="choice.level"
          :checked="level === choice.level"
          :label="choice.label"
          :description="choice.hint"
          :disabled="choice.disabled"
          @select="level = choice.level" />
      </div>
      <DbSelect
        v-model="where"
        label="Where"
        :options="scopeOptions.map(({ value, label }) => ({ value, label }))" />
      <div class="flex flex-col gap-1.5">
        <p class="text-sm font-medium text-ink-muted">
          Expires
          <span class="font-normal text-ink-faint"
            >(at most {{ maxDays }} days)</span
          >
        </p>
        <DbSegment
          v-model="expiry"
          class="self-start"
          :options="expiryOptions"
          label="Expires" />
        <DbInput
          v-if="expiry === 'custom'"
          v-model="customDays"
          class="max-w-[12rem]"
          label="Days"
          type="number"
          :min="1"
          :max="maxDays"
          :step="1" />
        <p v-if="expiryError" class="text-xs text-crit">{{ expiryError }}</p>
      </div>
      <DbAlert v-if="error">{{ error }}</DbAlert>
    </form>
    <template #footer>
      <DbButton variant="ghost" :disabled="saving" @click="emit('close')">
        Cancel
      </DbButton>
      <DbButton
        variant="primary"
        type="submit"
        form="agent-token"
        :loading="saving"
        data-testid="agent-token-submit">
        Create token
      </DbButton>
    </template>
  </DbModal>
</template>
