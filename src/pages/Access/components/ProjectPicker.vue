<script setup lang="ts">
import { computed } from "vue";
import { DbMenu, DbMenuItem, DbSearchField } from "~/components/ui";
import { ChevronDownIcon, FolderIcon } from "~/assets/icons";
import {
  projectMeta,
  type ProjectChoice,
} from "~/pages/Access/GiveAccess.controller";

/**
 * ProjectPicker — the project field on the Give access page. A button shows
 * the chosen project and what they hold there; it opens a list as wide as
 * the field, with a search box once there are more than six projects.
 * Presentation-only: the page filters, and it reports the id picked.
 */
const props = defineProps<{
  /** Every project they can be given access in. */
  choices: ProjectChoice[];
  /** The ones that match `filter`. */
  visible: ProjectChoice[];
  modelValue: string;
}>();

const emit = defineEmits<{ "update:modelValue": [id: string] }>();

const filter = defineModel<string>("filter", { required: true });

const chosen = computed(
  () =>
    props.choices.find((choice) => choice.project.id === props.modelValue) ??
    null,
);

function pick(id: string): void {
  filter.value = "";
  emit("update:modelValue", id);
}
</script>

<template>
  <DbMenu align="start" match-width>
    <template #trigger="{ toggle, attrs, open }">
      <button
        type="button"
        class="flex min-h-14 w-full cursor-pointer items-center gap-3 rounded-control border bg-raised px-3.5 py-2.5 text-left transition-colors hover:border-line-strong"
        :class="open ? 'border-accent/60' : 'border-line'"
        v-bind="attrs"
        aria-label="Project"
        data-testid="project-picker"
        @click="toggle">
        <FolderIcon class="h-4 w-4 shrink-0 text-ink-muted" />
        <span v-if="chosen" class="flex min-w-0 flex-1 flex-col">
          <span class="truncate font-mono text-sm font-medium text-ink-strong">
            {{ chosen.project.name }}
          </span>
          <span class="truncate text-[13px] text-ink-muted">
            {{ projectMeta(chosen) }}
          </span>
        </span>
        <span v-else class="flex-1 text-sm text-ink-faint">
          Choose a project
        </span>
        <ChevronDownIcon class="h-4 w-4 shrink-0 text-ink-faint" />
      </button>
    </template>
    <div v-if="choices.length > 6" class="p-1 pb-1.5">
      <DbSearchField
        v-model="filter"
        placeholder="Search projects"
        data-testid="project-search" />
    </div>
    <DbMenuItem
      v-for="choice in visible"
      :key="choice.project.id"
      :selected="choice.project.id === modelValue"
      data-testid="project-choice"
      @select="pick(choice.project.id)">
      <template #icon><FolderIcon class="h-4 w-4" /></template>
      <span class="font-mono">{{ choice.project.name }}</span>
      <template #trailing>
        <span class="shrink-0 text-[13px] font-normal text-ink-faint">
          {{ projectMeta(choice) }}
        </span>
      </template>
    </DbMenuItem>
    <p
      v-if="visible.length === 0"
      class="px-2.5 py-3 text-center text-sm text-ink-muted">
      No projects match "{{ filter.trim() }}".
    </p>
  </DbMenu>
</template>
