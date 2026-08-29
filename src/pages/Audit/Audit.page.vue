<script setup lang="ts">
import { computed, ref } from "vue";
import { useAuditController } from "./Audit.controller";
import { DashboardLayout } from "~/layouts";
import {
  DbAlert,
  DbBadge,
  DbButton,
  DbEmptyState,
  DbInput,
  DbSelect,
  DbSpinner,
} from "~/components/ui";
import { HistoryIcon, RefreshIcon } from "~/assets/icons";
import { formatRelativeTime, formatDateTime } from "~/utils/format";
import type { AuditEvent } from "~/services";

/**
 * Audit — the append-only trail of every administrative and runner action.
 * Cursor-paginated, filterable, and expandable per row for IDs and
 * metadata; project/environment names resolve from loaded lists while the
 * immutable IDs stay visible in the detail view.
 */
const controller = useAuditController();
// Destructure so refs auto-unwrap in the template (filters is reactive).
const {
  items,
  nextCursor,
  loading,
  loadingMore,
  loadError,
  hasLoaded,
  filters,
  projects,
  environments,
  load,
  loadMore,
} = controller;

const expandedId = ref<string | null>(null);

function toggle(event: AuditEvent): void {
  expandedId.value = expandedId.value === event.id ? null : event.id;
}

const projectOptions = computed(() => [
  { label: "All projects", value: "" },
  ...projects.value.map((project) => ({
    label: project.name,
    value: project.id,
  })),
]);

const environmentOptions = computed(() => [
  { label: "All environments", value: "" },
  ...environments.value.map((environment) => ({
    label: `${environment.projectName}/${environment.name}`,
    value: environment.id,
  })),
]);

const projectName = (id: string | null): string =>
  projects.value.find((project) => project.id === id)?.name ?? id ?? "—";
const environmentName = (id: string | null): string => {
  if (!id) return "—";
  const match = environments.value.find((environment) => environment.id === id);
  return match ? `${match.projectName}/${match.name}` : id;
};
</script>

<template>
  <DashboardLayout>
    <div class="mx-auto max-w-5xl p-8">
      <header class="mb-6">
        <h1 class="text-lg font-semibold">Audit</h1>
        <p class="mt-0.5 text-sm text-ink-muted">
          Immutable record of every action on this server.
        </p>
      </header>

      <!-- Filters -->
      <div class="mb-4 grid grid-cols-1 gap-3 sm:grid-cols-2 lg:grid-cols-4">
        <DbInput
          v-model="filters.action"
          label="Action"
          name="filter-action"
          placeholder="secret.revealed"
          mono />
        <DbSelect
          v-model="filters.projectId"
          label="Project"
          name="filter-project"
          :options="projectOptions" />
        <DbSelect
          v-model="filters.environmentId"
          label="Environment"
          name="filter-environment"
          :options="environmentOptions" />
        <DbInput
          v-model="filters.actor"
          label="Actor"
          name="filter-actor"
          placeholder="admin or runner id"
          mono />
      </div>

      <div class="mb-4 flex items-center gap-2">
        <DbButton
          size="sm"
          variant="secondary"
          :loading="loading"
          @click="load()">
          <RefreshIcon class="h-3.5 w-3.5" />
          Refresh
        </DbButton>
      </div>

      <DbAlert v-if="loadError" class="mb-4">
        {{ loadError }}
      </DbAlert>

      <DbSpinner v-if="loading" class="mx-auto mt-12 h-6 w-6 text-ink-muted" />

      <DbEmptyState
        v-else-if="hasLoaded && items.length === 0"
        title="No audit events"
        description="No events match the current filters. Actions appear here as projects, secrets, tokens, and sessions change.">
        <template #icon>
          <HistoryIcon class="h-5 w-5" />
        </template>
      </DbEmptyState>

      <div
        v-else
        class="overflow-x-auto rounded-[var(--radius-card)] border border-line bg-panel">
        <table class="min-w-full text-left text-sm" data-testid="audit-table">
          <thead>
            <tr class="border-b border-line">
              <th
                class="px-4 py-2.5 text-xs font-medium uppercase tracking-wide text-ink-muted">
                Time
              </th>
              <th
                class="px-4 py-2.5 text-xs font-medium uppercase tracking-wide text-ink-muted">
                Action
              </th>
              <th
                class="px-4 py-2.5 text-xs font-medium uppercase tracking-wide text-ink-muted">
                Actor
              </th>
              <th
                class="px-4 py-2.5 text-xs font-medium uppercase tracking-wide text-ink-muted">
                Context
              </th>
            </tr>
          </thead>
          <tbody>
            <template v-for="event in items" :key="event.id">
              <tr
                class="cursor-pointer border-b border-line-soft transition-colors last:border-b-0 hover:bg-raised/60"
                @click="toggle(event)">
                <td
                  class="whitespace-nowrap px-4 py-2.5 text-sm text-ink-muted"
                  :title="formatDateTime(event.createdAt)">
                  {{ formatRelativeTime(event.createdAt) }}
                </td>
                <td class="px-4 py-2.5">
                  <DbBadge tone="accent">{{ event.action }}</DbBadge>
                </td>
                <td class="px-4 py-2.5 text-sm">
                  <span class="font-mono text-ink">
                    {{ event.actorLabel ?? event.actorType }}
                  </span>
                </td>
                <td class="px-4 py-2.5 font-mono text-sm text-ink-muted">
                  {{ projectName(event.projectId) }}
                  <template v-if="event.environmentId">
                    / {{ environmentName(event.environmentId) }}
                  </template>
                </td>
              </tr>
              <tr
                v-if="expandedId === event.id"
                class="border-b border-line-soft bg-canvas/60 last:border-b-0">
                <td colspan="4" class="px-4 py-3">
                  <div class="flex flex-col gap-2 font-mono text-xs text-ink">
                    <p>
                      <span class="text-ink-muted">event:</span>
                      {{ event.id }}
                    </p>
                    <p v-if="event.actorId">
                      <span class="text-ink-muted">actor id:</span>
                      {{ event.actorId }}
                    </p>
                    <p v-if="event.resourceType">
                      <span class="text-ink-muted">resource:</span>
                      {{ event.resourceType }}
                      <template v-if="event.resourceId">
                        ({{ event.resourceId }})
                      </template>
                    </p>
                    <details v-if="Object.keys(event.metadata).length > 0">
                      <summary class="cursor-pointer text-ink-muted">
                        metadata
                      </summary>
                      <pre
                        class="mt-2 overflow-x-auto rounded border border-line-soft bg-panel px-3 py-2 text-xs leading-relaxed text-ink"
                        >{{ JSON.stringify(event.metadata, null, 2) }}</pre>
                    </details>
                  </div>
                </td>
              </tr>
            </template>
          </tbody>
        </table>
      </div>

      <div v-if="nextCursor" class="mt-4 flex justify-center">
        <DbButton
          variant="secondary"
          :loading="loadingMore"
          @click="loadMore()">
          Load more
        </DbButton>
      </div>
    </div>
  </DashboardLayout>
</template>
