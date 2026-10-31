<script setup lang="ts">
import { computed, toRef } from "vue";
import {
  useGiveAccessDialog,
  type GrantPrincipal,
} from "./GiveAccessDialog.controller";
import {
  DbAlert,
  DbButton,
  DbModal,
  DbRadioCard,
  DbSegment,
  DbSelect,
  DbSkeleton,
} from "~/components/ui";
import type { Grant } from "~/services/access.api";
import { formatLevel } from "~/utils/format";

/**
 * GiveAccessDialog — gives a person, AI agent or group access to a
 * project or one environment, or changes the level of a grant they hold.
 * Used from the user, agent and group pages.
 */
const props = withDefaults(
  defineProps<{
    open: boolean;
    principal: GrantPrincipal;
    /** Their current grants, to say when a new one replaces one. */
    existing?: Grant[];
    /** Set to change this grant's level instead of giving a new one. */
    editing?: Grant | null;
  }>(),
  { existing: () => [], editing: null },
);

const emit = defineEmits<{ close: []; saved: [] }>();

const dialog = useGiveAccessDialog(
  toRef(props, "open"),
  toRef(props, "principal"),
  toRef(props, "existing"),
  toRef(props, "editing"),
);
const {
  projects,
  manageable,
  projectId,
  where,
  environments,
  environmentId,
  level,
  levels,
  summary,
  saving,
  error,
} = dialog;

const title = computed(() =>
  props.editing
    ? `Change ${props.principal.label}'s access`
    : `Give ${props.principal.label} access`,
);

const projectOptions = computed(() =>
  manageable.value.map((project) => ({
    label: project.name,
    value: project.id,
  })),
);
const environmentOptions = computed(() =>
  environments.value.map((environment) => ({
    label: environment.protected
      ? `${environment.name} (protected)`
      : environment.name,
    value: environment.id,
  })),
);
const whereOptions = [
  { value: "project", label: "Whole project" },
  { value: "environment", label: "One environment" },
] as const;

async function save(): Promise<void> {
  if (await dialog.submit()) emit("saved");
}
</script>

<template>
  <DbModal :open="open" :title="title" size="lg" @close="emit('close')">
    <form
      id="give-access"
      class="flex flex-col gap-4"
      novalidate
      data-testid="give-access-dialog"
      @submit.prevent="save">
      <template v-if="editing">
        <p class="text-sm text-ink">
          <span class="font-mono text-ink-strong">{{
            editing.scopeLabel
          }}</span>
          <span class="text-ink-muted">
            ·
            {{
              editing.scopeType === "environment"
                ? "one environment"
                : "whole project"
            }}
          </span>
        </p>
      </template>
      <template v-else>
        <div
          v-if="projects === null"
          class="flex flex-col gap-2"
          data-testid="give-access-loading">
          <DbSkeleton class="h-14 w-full rounded-control" />
        </div>
        <DbAlert v-else-if="manageable.length === 0" tone="info">
          You don't manage any project, so you can't give access from here.
          Project managers give access to their projects.
        </DbAlert>
        <template v-else>
          <DbSelect
            v-model="projectId"
            label="Project"
            :options="projectOptions" />
          <div class="flex flex-col gap-1.5">
            <p class="text-sm font-medium text-ink-muted">Where</p>
            <DbSegment
              v-model="where"
              class="self-start"
              :options="whereOptions"
              label="Where" />
          </div>
          <DbSelect
            v-if="where === 'environment'"
            v-model="environmentId"
            label="Environment"
            :options="environmentOptions"
            :disabled="environmentOptions.length === 0" />
        </template>
      </template>

      <div
        v-if="editing || manageable.length > 0"
        class="flex flex-col gap-1.5"
        role="radiogroup"
        aria-label="Level">
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

      <DbAlert
        v-if="editing || manageable.length > 0"
        tone="info"
        data-testid="give-access-summary">
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
        form="give-access"
        :loading="saving"
        :disabled="!editing && manageable.length === 0"
        data-testid="give-access-submit">
        {{ editing ? "Save" : "Give access" }}
      </DbButton>
    </template>
  </DbModal>
</template>
