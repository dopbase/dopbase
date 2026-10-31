<script setup lang="ts">
import { computed, toRef } from "vue";
import {
  MAX_COMPARE,
  useComparePanelController,
  type CompareColumn,
} from "./ComparePanel.controller";
import CompareCell from "./CompareCell.vue";
import FillSecretDialog from "./FillSecretDialog.vue";
import {
  DbAlert,
  DbBadge,
  DbButton,
  DbEmptyState,
  DbMenu,
  DbMenuItem,
  DbSearchField,
  DbSegment,
  DbSkeleton,
} from "~/components/ui";
import {
  ChevronRightIcon,
  LayersIcon,
  LockIcon,
  PlusIcon,
  XIcon,
} from "~/assets/icons";
import type { Environment } from "~/services";

/**
 * ComparePanel — the project page's Compare tab. The environment open in
 * the context bar is the base, and up to three others are read against
 * it, key by key. Cells say Same, Differs, Not set or Set; values are
 * never shown. Click a Not set cell to match another environment or give
 * it a value of its own.
 */
const props = defineProps<{
  environment: Environment;
  environments: Environment[];
  /** Offer New environment when the project has only this one. */
  canCreateEnvironment: boolean;
}>();

const emit = defineEmits<{ "new-environment": [] }>();

const controller = useComparePanelController(
  toRef(props, "environment"),
  toRef(props, "environments"),
);
const {
  others,
  selected,
  addable,
  canAdd,
  result,
  loading,
  loadError,
  baseColumn,
  columns,
  namesOnly,
  rows,
  view,
  filter,
  differences,
  visibleRows,
  summary,
  fillDialog,
  filling,
  fillError,
  filled,
} = controller;

const base = computed(() => props.environment);

const viewOptions = computed(() => [
  { value: "differences", label: `Differences ${differences.value.length}` },
  { value: "all", label: `All keys ${rows.value.length}` },
]);

/** Key column, then the base and each compared column. */
const gridColumns = computed(
  () =>
    `minmax(180px, 1.3fr) repeat(${columns.value.length + 1}, minmax(160px, 1fr))`,
);
/** Below this the table scrolls sideways inside its card. */
const gridMinWidth = computed(
  () =>
    `${180 + (columns.value.length + 1) * 160 + 12 * (columns.value.length + 1) + 16}px`,
);

const comparedNames = computed(() => {
  const names = columns.value.map((column) => column.name);
  if (names.length <= 1) return names.join("");
  return `${names.slice(0, -1).join(", ")} and ${names.at(-1)}`;
});

const namesOnlyNote = computed(() => {
  const names = namesOnly.value.map((column) => column.name);
  if (names.length === 0) return "";
  const list =
    names.length === 1
      ? names[0]
      : `${names.slice(0, -1).join(", ")} and ${names.at(-1)}`;
  return `You see names only in ${list}, so ${names.length === 1 ? "its column shows" : "their columns show"} which keys exist there, not whether the values match ${base.value.name}.`;
});

function setChoice(choice: string): void {
  if (fillDialog.value) fillDialog.value.choice = choice;
}
function setValue(value: string): void {
  if (fillDialog.value) fillDialog.value.value = value;
}

const fillLabel = (key: string, environment: string) =>
  `Set ${key} in ${environment}`;

/** The base's Secrets tab, where values can be seen and changed. */
const secretsLink = computed(() => ({
  name: "environment",
  params: {
    projectRef: props.environment.projectName,
    environmentName: props.environment.name,
  },
}));

const columnIcon = (column: CompareColumn) =>
  column.protected ? LockIcon : LayersIcon;
</script>

<template>
  <div class="flex flex-col gap-4" data-testid="compare-panel">
    <DbEmptyState
      v-if="others.length === 0"
      class="mx-auto w-full max-w-[560px]"
      title="Nothing to compare yet"
      :description="`${environment.projectName} only has ${environment.name}. Add another environment, such as staging, and its keys line up here against ${environment.name}.`">
      <template #icon><LayersIcon class="h-5 w-5" /></template>
      <template v-if="canCreateEnvironment" #actions>
        <DbButton variant="primary" @click="emit('new-environment')">
          <PlusIcon class="h-4 w-4" />
          New environment
        </DbButton>
      </template>
    </DbEmptyState>

    <template v-else>
      <header class="flex flex-col gap-1">
        <h2 class="text-[15px] font-semibold text-ink-strong">
          Compare {{ environment.name }} with other environments
        </h2>
        <p class="text-sm text-ink-muted">
          {{ environment.name }} is the base because it's open in the switcher
          above. Switch environments to compare from another one.
        </p>
      </header>

      <div
        class="flex flex-wrap items-center gap-2"
        data-testid="compare-toolbar">
        <span class="text-sm text-ink-muted">Compare with</span>
        <span
          v-for="other in selected"
          :key="other.id"
          class="inline-flex h-8 items-center gap-1.5 rounded-control border border-accent/50 bg-accent-soft pl-2.5 pr-1 font-mono text-sm text-ink-strong"
          data-testid="compare-chip">
          <LockIcon
            v-if="other.protected"
            class="h-3.5 w-3.5 text-warn"
            aria-label="protected" />
          {{ other.name }}
          <button
            type="button"
            class="flex h-6 w-6 cursor-pointer items-center justify-center rounded-control text-ink-muted transition-colors hover:bg-raised hover:text-ink-strong pointer-coarse:h-8 pointer-coarse:w-8"
            :aria-label="`Stop comparing with ${other.name}`"
            @click="controller.remove(other.id)">
            <XIcon class="h-3.5 w-3.5" />
          </button>
        </span>
        <DbMenu v-if="canAdd" align="start">
          <template #trigger="{ toggle, attrs }">
            <button
              type="button"
              class="inline-flex h-8 cursor-pointer items-center gap-1.5 rounded-control border border-dashed border-line-strong px-2.5 text-sm text-ink-muted transition-colors hover:bg-raised hover:text-ink-strong"
              v-bind="attrs"
              data-testid="compare-add"
              @click="toggle">
              <PlusIcon class="h-3.5 w-3.5" />
              Add environment
            </button>
          </template>
          <DbMenuItem
            v-for="candidate in addable"
            :key="candidate.id"
            :title="candidate.name"
            @select="controller.add(candidate.id)">
            <template #icon>
              <LockIcon v-if="candidate.protected" class="h-4 w-4 text-warn" />
              <LayersIcon v-else class="h-4 w-4" />
            </template>
            <span class="font-mono">{{ candidate.name }}</span>
          </DbMenuItem>
        </DbMenu>
        <span
          v-else-if="addable.length > 0"
          class="text-xs text-ink-faint"
          data-testid="compare-limit">
          {{ MAX_COMPARE }} at a time. Remove one to compare another.
        </span>

        <div
          class="flex w-full flex-wrap items-center gap-2 lg:ml-auto lg:w-auto">
          <DbSearchField
            v-model="filter"
            class="w-full sm:w-56"
            placeholder="Filter keys" />
          <DbSegment
            v-model="view"
            :options="viewOptions"
            label="Which keys to show" />
        </div>
      </div>

      <div
        v-if="result && selected.length > 0"
        class="flex flex-wrap gap-x-5 gap-y-1.5 text-[13px] text-ink"
        data-testid="compare-summary">
        <span class="inline-flex items-center gap-1.5">
          <span
            class="h-2.5 w-2.5 rounded-[2px] border border-dashed border-crit"
            aria-hidden="true" />
          {{ summary.missingInBase }} not set in {{ environment.name }}
        </span>
        <span class="inline-flex items-center gap-1.5">
          <span
            class="h-2.5 w-2.5 rounded-[2px] border border-dashed border-crit"
            aria-hidden="true" />
          {{ summary.missingElsewhere }} missing in {{ comparedNames }}
        </span>
        <span class="inline-flex items-center gap-1.5">
          <span
            class="h-2.5 w-2.5 rounded-[2px] bg-accent"
            aria-hidden="true" />
          {{ summary.differs }}
          {{ summary.differs === 1 ? "key differs" : "keys differ" }} from
          {{ environment.name }}
        </span>
        <span class="inline-flex items-center gap-1.5">
          <span
            class="h-2.5 w-2.5 rounded-[2px] bg-line-strong"
            aria-hidden="true" />
          {{ summary.same }} the same everywhere
        </span>
      </div>

      <DbAlert v-if="namesOnlyNote" tone="info">{{ namesOnlyNote }}</DbAlert>
      <DbAlert v-if="filled" tone="success" data-testid="compare-filled">
        {{ filled }}
      </DbAlert>

      <div v-if="loadError" class="flex flex-col items-start gap-3">
        <DbAlert>{{ loadError }}</DbAlert>
        <DbButton size="sm" @click="controller.load">Try again</DbButton>
      </div>

      <DbEmptyState
        v-else-if="selected.length === 0"
        title="Pick an environment to compare with"
        :description="`Add one above, and its keys line up against ${environment.name}.`">
        <template #icon><LayersIcon class="h-5 w-5" /></template>
      </DbEmptyState>

      <div
        v-else-if="!result"
        class="flex flex-col gap-2"
        data-testid="compare-skeleton">
        <DbSkeleton
          v-for="line in 5"
          :key="line"
          class="h-11 w-full rounded-control" />
      </div>

      <DbEmptyState
        v-else-if="view === 'differences' && differences.length === 0"
        :title="`${environment.name} matches ${comparedNames}`"
        :description="`All ${rows.length} keys are set everywhere, with matching values where they're compared. Switch to All keys to see them.`">
        <template #icon><LayersIcon class="h-5 w-5" /></template>
      </DbEmptyState>

      <template v-else>
        <!-- Compare is read-only for set values: the one rule worth stating
             above the table, with the way to where values can be seen. -->
        <DbAlert
          tone="info"
          title="Compare doesn't show or change values that are already set."
          data-testid="compare-policy">
          For security, seeing or editing a value happens in Secrets, where
          revealing asks for your password and is recorded in the audit log.
          Here you can only fill keys that are Not set.
          <RouterLink
            :to="secretsLink"
            class="whitespace-nowrap font-medium text-accent-strong underline underline-offset-2 hover:no-underline">
            Open {{ environment.name }}'s Secrets ›
          </RouterLink>
        </DbAlert>

        <!-- Table on wider screens -->
        <div
          class="hidden overflow-x-auto rounded-card border border-line bg-panel sm:block"
          :class="loading ? 'opacity-70' : ''"
          :aria-busy="loading">
          <div
            class="grid w-full items-center gap-x-3 text-sm"
            :style="{
              gridTemplateColumns: gridColumns,
              minWidth: gridMinWidth,
            }"
            role="table"
            aria-label="Keys compared across environments"
            data-testid="compare-table">
            <div class="contents" role="row">
              <div
                class="flex h-11 items-center border-b border-line-soft bg-raised/50 pl-4 font-medium text-ink-muted"
                role="columnheader">
                Key
              </div>
              <div
                v-if="baseColumn"
                class="flex h-11 items-center gap-1.5 border-b border-line-soft bg-raised/50"
                role="columnheader">
                <component
                  :is="columnIcon(baseColumn)"
                  class="h-3.5 w-3.5 shrink-0"
                  :class="
                    baseColumn.protected ? 'text-warn' : 'text-ink-muted'
                  " />
                <span class="truncate font-mono font-medium text-ink-strong">
                  {{ baseColumn.name }}
                </span>
                <DbBadge tone="accent">Base</DbBadge>
              </div>
              <div
                v-for="column in columns"
                :key="column.id"
                class="flex h-11 items-center gap-1.5 border-b border-line-soft bg-raised/50 last:pr-4"
                role="columnheader">
                <component
                  :is="columnIcon(column)"
                  class="h-3.5 w-3.5 shrink-0"
                  :class="column.protected ? 'text-warn' : 'text-ink-muted'" />
                <span
                  class="truncate font-mono font-medium text-ink"
                  :title="column.name">
                  {{ column.name }}
                </span>
                <span
                  v-if="baseColumn?.valuesCompared && !column.valuesCompared"
                  class="shrink-0 text-xs text-ink-faint">
                  Names only
                </span>
              </div>
            </div>

            <div
              v-for="row in visibleRows"
              :key="row.key"
              class="contents"
              role="row"
              data-testid="compare-row">
              <div
                class="flex h-[52px] min-w-0 items-center border-b border-line-soft pl-4"
                role="cell">
                <span
                  class="truncate font-mono text-ink-strong"
                  :title="row.key">
                  {{ row.key }}
                </span>
              </div>
              <div
                v-for="cell in [row.base, ...row.cells]"
                :key="cell.column.id"
                class="border-b border-line-soft py-2.5 last:pr-4"
                role="cell">
                <CompareCell
                  :status="cell.status"
                  :version="cell.version"
                  :fillable="cell.fillable"
                  :aria-label="
                    cell.fillable
                      ? fillLabel(row.key, cell.column.name)
                      : undefined
                  "
                  @click="controller.openFill(row, cell)" />
              </div>
            </div>
          </div>
          <p
            v-if="visibleRows.length === 0"
            class="px-4 py-6 text-center text-sm text-ink-muted">
            No keys match "{{ filter.trim() }}".
          </p>
          <button
            v-if="view === 'differences' && summary.same > 0"
            type="button"
            class="flex w-full cursor-pointer items-center gap-2 px-4 py-3 text-left text-sm text-ink-muted transition-colors hover:bg-raised hover:text-ink"
            data-testid="compare-show-same"
            @click="view = 'all'">
            <ChevronRightIcon class="h-4 w-4" />
            {{ summary.same }}
            {{ summary.same === 1 ? "key is" : "keys are" }} the same in every
            environment
          </button>
        </div>

        <!-- Cards on phones -->
        <div class="flex flex-col gap-3 sm:hidden" data-testid="compare-cards">
          <div
            v-for="row in visibleRows"
            :key="row.key"
            class="flex flex-col gap-2 rounded-card border border-line bg-panel px-3 pb-3 pt-2.5">
            <div class="flex items-center gap-2">
              <span
                class="min-w-0 flex-1 font-mono text-sm font-medium text-ink-strong [overflow-wrap:anywhere]">
                {{ row.key }}
              </span>
            </div>
            <div
              v-for="cell in [row.base, ...row.cells]"
              :key="cell.column.id"
              class="flex items-center gap-2">
              <span
                class="flex w-32 min-w-0 shrink-0 items-center gap-1.5"
                :title="cell.column.name">
                <span
                  class="truncate font-mono text-sm"
                  :class="
                    cell === row.base ? 'text-ink-strong' : 'text-ink-muted'
                  ">
                  {{ cell.column.name }}
                </span>
                <DbBadge v-if="cell === row.base" tone="accent">Base</DbBadge>
              </span>
              <CompareCell
                class="flex-1"
                :status="cell.status"
                :version="cell.version"
                :fillable="cell.fillable"
                :aria-label="
                  cell.fillable
                    ? fillLabel(row.key, cell.column.name)
                    : undefined
                "
                @click="controller.openFill(row, cell)" />
            </div>
          </div>
          <button
            v-if="view === 'differences' && summary.same > 0"
            type="button"
            class="flex cursor-pointer items-center gap-2 py-2 text-sm text-ink-muted"
            @click="view = 'all'">
            <ChevronRightIcon class="h-4 w-4" />
            {{ summary.same }}
            {{ summary.same === 1 ? "key is" : "keys are" }} the same everywhere
          </button>
        </div>
      </template>
    </template>

    <FillSecretDialog
      :dialog="fillDialog"
      :filling="filling"
      :error="fillError"
      @update:choice="setChoice"
      @update:value="setValue"
      @confirm="controller.confirmFill"
      @close="controller.cancelFill" />
  </div>
</template>
