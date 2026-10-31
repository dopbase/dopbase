<script setup lang="ts">
import { toRef } from "vue";
import {
  usePersonalTokenDialog,
  type TokenLimits,
  type TokenPreset,
} from "./PersonalTokenDialog.controller";
import {
  DbAlert,
  DbButton,
  DbInput,
  DbModal,
  DbRadioCard,
  DbSegment,
  DbSelect,
} from "~/components/ui";
import { InfoIcon } from "~/assets/icons";
import type { CreatedPersonalToken } from "~/services/personal-tokens.api";
import type { Environment } from "~/services/environments.api";
import type { Project } from "~/services/projects.api";

/**
 * PersonalTokenDialog — New personal token, or New audit token. Emits the
 * created token so the tab can show it once.
 */
const props = defineProps<{
  open: boolean;
  auditOnly: boolean;
  projects: Project[];
  environments: Environment[];
  limits: TokenLimits;
}>();

const emit = defineEmits<{
  close: [];
  created: [token: CreatedPersonalToken];
}>();

const dialog = usePersonalTokenDialog(
  toRef(props, "open"),
  toRef(props, "auditOnly"),
  toRef(props, "projects"),
  toRef(props, "environments"),
  toRef(props, "limits"),
);
const {
  preset,
  presets,
  name,
  level,
  levels,
  where,
  scopeOptions,
  expiry,
  expiryOptions,
  customDays,
  maxDays,
  summary,
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
    :title="auditOnly ? 'New audit token' : 'New personal token'"
    size="lg"
    @close="emit('close')">
    <form
      id="personal-token"
      class="flex flex-col gap-4"
      novalidate
      data-testid="personal-token-dialog"
      @submit.prevent="save">
      <div v-if="!auditOnly" class="flex flex-col gap-1.5">
        <p class="text-sm font-medium text-ink-muted">Start from</p>
        <DbSegment
          :model-value="preset"
          class="self-start"
          :options="presets"
          label="Start from"
          @update:model-value="
            (value: string) => dialog.choosePreset(value as TokenPreset)
          " />
      </div>
      <DbInput
        v-model="name"
        label="Name"
        name="tokenName"
        :placeholder="auditOnly ? 'splunk-forwarder' : 'claude-laptop'"
        mono
        :error="nameError"
        hint="Name it after where it's used." />
      <div
        v-if="!auditOnly"
        class="flex flex-col gap-1.5"
        role="radiogroup"
        aria-label="Can">
        <p class="text-sm font-medium text-ink-muted">Can</p>
        <DbRadioCard
          v-for="choice in levels"
          :key="choice.level"
          name="personal-token-level"
          :value="choice.level"
          :checked="level === choice.level"
          :label="choice.label"
          :description="choice.hint"
          :disabled="choice.disabled"
          @select="level = choice.level" />
      </div>
      <DbSelect
        v-if="!auditOnly"
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
      <p
        class="flex items-start gap-2.5 rounded-control border border-line-soft bg-editor px-3 py-2.5 text-sm text-ink"
        data-testid="personal-token-summary">
        <InfoIcon class="mt-0.5 h-4 w-4 shrink-0 text-accent-strong" />
        <span>{{ summary }}</span>
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
        form="personal-token"
        :loading="saving"
        data-testid="personal-token-submit">
        Create token
      </DbButton>
    </template>
  </DbModal>
</template>
