<script setup lang="ts">
import { computed, toRef } from "vue";
import { useChangeAccessDialog } from "./ChangeAccessDialog.controller";
import { DbAlert, DbButton, DbModal, DbRadioCard } from "~/components/ui";
import type { Grant } from "~/services/access.api";
import { formatLevel } from "~/utils/format";
import type { GrantPrincipal } from "~/utils/level";

/**
 * ChangeAccessDialog — changes the level of a grant a person, AI agent or
 * group already holds. Open while `grant` is set.
 */
const props = defineProps<{
  principal: GrantPrincipal;
  grant: Grant | null;
}>();

const emit = defineEmits<{ close: []; saved: [] }>();

const dialog = useChangeAccessDialog(
  toRef(props, "principal"),
  toRef(props, "grant"),
);
const { level, levels, summary, saving, error } = dialog;

const title = computed(() => `Change ${props.principal.label}'s access`);

async function save(): Promise<void> {
  if (await dialog.submit()) emit("saved");
}
</script>

<template>
  <DbModal
    :open="grant !== null"
    :title="title"
    size="lg"
    @close="emit('close')">
    <form
      id="change-access"
      class="flex flex-col gap-4"
      novalidate
      data-testid="change-access-dialog"
      @submit.prevent="save">
      <p v-if="grant" class="text-sm text-ink">
        <span class="font-mono text-ink-strong">{{ grant.scopeLabel }}</span>
        <span class="text-ink-muted">
          ·
          {{
            grant.scopeType === "environment"
              ? "one environment"
              : "whole project"
          }}
        </span>
      </p>

      <div class="flex flex-col gap-1.5" role="radiogroup" aria-label="Level">
        <p class="text-sm font-medium text-ink-muted">
          Level
          <span v-if="principal.maxLevel" class="font-normal text-ink-faint">
            · their role allows up to {{ formatLevel(principal.maxLevel) }}
          </span>
        </p>
        <DbRadioCard
          v-for="choice in levels"
          :key="choice.level"
          name="grant-level"
          :value="choice.level"
          :checked="level === choice.level"
          :label="choice.label"
          :description="choice.hint"
          :disabled="choice.disabled"
          @select="level = choice.level" />
      </div>

      <DbAlert tone="info" data-testid="change-access-summary">
        {{ summary }}
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
        form="change-access"
        :loading="saving"
        data-testid="change-access-submit">
        Save
      </DbButton>
    </template>
  </DbModal>
</template>
