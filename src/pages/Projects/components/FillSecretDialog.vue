<script setup lang="ts">
import { computed } from "vue";
import {
  DbAlert,
  DbButton,
  DbModal,
  DbRadioCard,
  DbTextarea,
} from "~/components/ui";
import type { FillDialog } from "./ComparePanel.controller";

/**
 * FillSecretDialog — sets a key where Compare shows Not set. Either match
 * an environment that has it, copied on the server so nobody sees the
 * value, or type a value of its own. Renders props, reports choices.
 */
const props = defineProps<{
  dialog: FillDialog | null;
  filling: boolean;
  error: string | null;
}>();

const emit = defineEmits<{
  "update:choice": [choice: string];
  "update:value": [value: string];
  confirm: [];
  close: [];
}>();

const target = computed(() => props.dialog?.target.column.name ?? "");
const nextVersion = computed(() => (props.dialog?.target.version ?? 0) + 1);
</script>

<template>
  <DbModal
    :open="dialog !== null"
    :title="dialog ? `Set ${dialog.key} in ${target}` : ''"
    size="lg"
    @close="emit('close')">
    <form
      v-if="dialog"
      id="fill-secret"
      class="flex flex-col gap-3"
      data-testid="fill-dialog"
      novalidate
      @submit.prevent="emit('confirm')">
      <p class="text-sm leading-[22px] text-ink">
        {{ target }} doesn't have {{ dialog.key }}. Match an environment that
        does, or give it a value of its own.
      </p>
      <DbRadioCard
        v-for="source in dialog.sources"
        :key="source.column.id"
        name="fill-choice"
        :value="source.column.id"
        :checked="dialog.choice === source.column.id"
        :label="`Match ${source.column.name}`"
        :description="`Copies ${source.column.name}'s current value (v${source.version}) on the server. Nobody sees it.`"
        @select="emit('update:choice', $event)" />
      <DbRadioCard
        name="fill-choice"
        value="own"
        :checked="dialog.choice === 'own'"
        label="Use its own value"
        :description="`Type the value ${target} should use.`"
        @select="emit('update:choice', $event)" />
      <DbTextarea
        v-if="dialog.choice === 'own'"
        :model-value="dialog.value"
        label="Value"
        name="value"
        :rows="3"
        placeholder="paste the value"
        hint="Sent over HTTPS and stored encrypted."
        @update:model-value="emit('update:value', $event)" />
      <p class="text-[13px] text-ink-muted">
        Saved as v{{ nextVersion }} in {{ target }}. Apps pick it up on their
        next start, and the audit log records the change.
      </p>
      <DbAlert v-if="error">{{ error }}</DbAlert>
    </form>
    <template #footer>
      <DbButton variant="ghost" :disabled="filling" @click="emit('close')">
        Cancel
      </DbButton>
      <DbButton
        variant="primary"
        type="submit"
        form="fill-secret"
        :loading="filling"
        data-testid="fill-confirm">
        Save
      </DbButton>
    </template>
  </DbModal>
</template>
