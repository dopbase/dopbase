<script setup lang="ts">
import { nextTick, ref, toRef } from "vue";
import { useActivityPanelController } from "./ActivityPanel.controller";
import { DbAlert, DbButton, DbEmptyState, DbSkeleton } from "~/components/ui";
import {
  ActivityIcon,
  ExternalLinkIcon,
  LockIcon,
  PlusIcon,
  XIcon,
} from "~/assets/icons";
import type { Environment } from "~/services";

/**
 * ActivityPanel — the project's audit trail, narrowed to this environment
 * unless the environment chip is removed. Action and actor narrow it more.
 * The full audit log opens with the same filters.
 */
const props = defineProps<{
  projectId: string | null;
  projectName: string;
  environment: Environment | null;
  environments: Environment[];
}>();

const {
  wholeProject,
  action,
  actor,
  rows,
  loading,
  loadingMore,
  loadError,
  nextCursor,
  loadMore,
  auditLink,
} = useActivityPanelController(
  toRef(props, "projectId"),
  toRef(props, "environment"),
  toRef(props, "environments"),
);

/** Which "+ Action" / "+ Actor" chip is being typed into. */
const editing = ref<"action" | "actor" | null>(null);
const draft = ref("");
const draftInput = ref<HTMLInputElement | null>(null);

async function startEditing(field: "action" | "actor"): Promise<void> {
  editing.value = field;
  draft.value = field === "action" ? action.value : actor.value;
  await nextTick();
  draftInput.value?.focus();
}

function commit(): void {
  if (editing.value === "action") action.value = draft.value.trim();
  if (editing.value === "actor") actor.value = draft.value.trim();
  editing.value = null;
}

const chip =
  "inline-flex h-[30px] items-center gap-1.5 rounded-control border px-2.5 text-sm";
const activeChip = `${chip} border-accent/60 bg-accent-soft text-ink-strong`;
const addChip = `${chip} cursor-pointer border-line text-ink-muted transition-colors hover:border-line-strong hover:text-ink`;
</script>

<template>
  <div class="flex flex-col gap-4">
    <div
      class="flex flex-wrap items-center gap-2"
      data-testid="activity-filters">
      <span :class="activeChip">project: {{ projectName }}</span>
      <span v-if="environment && !wholeProject" :class="activeChip">
        environment: {{ environment.name }}
        <button
          type="button"
          class="-mr-1 flex cursor-pointer items-center justify-center p-1 text-ink-muted hover:text-ink-strong pointer-coarse:-my-2 pointer-coarse:p-3"
          aria-label="Show the whole project"
          @click="wholeProject = true">
          <XIcon class="h-3.5 w-3.5" />
        </button>
      </span>
      <button
        v-else-if="environment"
        type="button"
        :class="addChip"
        @click="wholeProject = false">
        <PlusIcon class="h-3.5 w-3.5" />
        Only {{ environment.name }}
      </button>

      <template v-for="field in ['action', 'actor'] as const" :key="field">
        <input
          v-if="editing === field"
          :ref="(element) => (draftInput = element as HTMLInputElement | null)"
          v-model="draft"
          class="h-[30px] w-48 rounded-control border border-accent/60 bg-panel px-2.5 font-mono text-sm text-ink-strong outline-none"
          :placeholder="
            field === 'action' ? 'secret.revealed' : 'maya@example.com'
          "
          :aria-label="field === 'action' ? 'Action' : 'Actor'"
          @keydown.enter.prevent="commit"
          @keydown.esc="editing = null"
          @blur="commit" />
        <span
          v-else-if="(field === 'action' ? action : actor) !== ''"
          :class="activeChip">
          {{ field }}: {{ field === "action" ? action : actor }}
          <button
            type="button"
            class="-mr-1 flex cursor-pointer items-center justify-center p-1 text-ink-muted hover:text-ink-strong pointer-coarse:-my-2 pointer-coarse:p-3"
            :aria-label="`Clear the ${field} filter`"
            @click="field === 'action' ? (action = '') : (actor = '')">
            <XIcon class="h-3.5 w-3.5" />
          </button>
        </span>
        <button
          v-else
          type="button"
          :class="addChip"
          @click="startEditing(field)">
          <PlusIcon class="h-3.5 w-3.5" />
          {{ field === "action" ? "Action" : "Actor" }}
        </button>
      </template>

      <RouterLink
        :to="auditLink"
        class="ml-auto inline-flex items-center gap-1.5 rounded-control px-2.5 py-1.5 text-sm font-medium text-ink transition-colors hover:bg-raised hover:text-ink-strong">
        <ExternalLinkIcon class="h-4 w-4" />
        Open in audit log
      </RouterLink>
    </div>

    <DbAlert v-if="loadError">{{ loadError }}</DbAlert>

    <div
      v-if="loading && rows.length === 0"
      class="overflow-hidden rounded-card border border-line bg-panel"
      data-testid="activity-skeleton">
      <div
        v-for="i in 5"
        :key="i"
        class="flex h-12 items-center gap-6 border-b border-line-soft px-4 last:border-b-0">
        <DbSkeleton class="h-4 w-16" />
        <DbSkeleton class="h-4 w-32" />
        <DbSkeleton class="h-4 w-40" />
      </div>
    </div>

    <DbEmptyState
      v-else-if="rows.length === 0 && !loadError"
      title="Nothing here yet"
      description="Reads, changes and access updates show up here as they happen. You see the events your audit access covers.">
      <template #icon>
        <ActivityIcon class="h-5 w-5" />
      </template>
    </DbEmptyState>

    <div
      v-else-if="rows.length > 0"
      class="overflow-x-auto rounded-card border border-line bg-panel">
      <table class="min-w-full text-left text-sm" data-testid="activity-table">
        <thead>
          <tr class="border-b border-line-soft">
            <th class="h-11 px-4 font-medium text-ink-muted">Time</th>
            <th class="h-11 px-4 font-medium text-ink-muted">Action</th>
            <th
              class="hidden h-11 px-4 font-medium text-ink-muted sm:table-cell">
              Actor
            </th>
            <th
              class="hidden h-11 px-4 font-medium text-ink-muted md:table-cell">
              Where
            </th>
            <th
              class="hidden h-11 px-4 font-medium text-ink-muted lg:table-cell">
              Details
            </th>
          </tr>
        </thead>
        <tbody>
          <tr
            v-for="row in rows"
            :key="row.event.id"
            class="border-b border-line-soft last:border-b-0"
            data-testid="activity-row">
            <td
              class="h-12 whitespace-nowrap px-4 align-top text-ink sm:align-middle">
              <span class="inline-block pt-2.5 sm:pt-0">{{ row.time }}</span>
            </td>
            <td class="h-12 max-w-0 px-4 py-2 sm:max-w-none">
              <span
                class="whitespace-nowrap rounded-control bg-accent-soft px-2 py-0.5 font-mono text-sm text-accent-strong">
                {{ row.event.action }}
              </span>
              <!-- Phones hide the Actor and Where columns; keep both here. -->
              <p
                class="mt-1 truncate font-mono text-[13px] text-ink-muted sm:hidden"
                data-testid="activity-row-meta">
                {{ row.actor
                }}<template v-if="row.where"> · {{ row.where }}</template>
              </p>
            </td>
            <td
              class="hidden h-12 max-w-[16rem] truncate px-4 font-mono text-sm text-ink sm:table-cell">
              {{ row.actor }}
            </td>
            <td class="hidden h-12 px-4 md:table-cell">
              <span
                v-if="row.where"
                class="inline-flex items-center gap-1.5 rounded-control border border-line px-2 py-0.5 font-mono text-sm text-ink-strong">
                {{ row.where }}
                <LockIcon
                  v-if="row.whereProtected"
                  class="h-3.5 w-3.5 text-warn"
                  aria-label="protected" />
              </span>
              <span v-else class="text-ink-muted">Project</span>
            </td>
            <td
              class="hidden h-12 max-w-[20rem] truncate px-4 font-mono text-[13px] text-ink-muted lg:table-cell"
              :title="row.details">
              {{ row.details }}
            </td>
          </tr>
        </tbody>
      </table>
    </div>

    <div v-if="nextCursor" class="flex justify-center">
      <DbButton :loading="loadingMore" @click="loadMore">Load more</DbButton>
    </div>
  </div>
</template>
