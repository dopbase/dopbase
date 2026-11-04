<script setup lang="ts">
import { useAuditController } from "./Audit.controller";
import { DashboardLayout } from "~/layouts";
import PageHeader from "~/components/app/PageHeader.vue";
import {
  DbAlert,
  DbBadge,
  DbButton,
  DbCopyButton,
  DbEmptyState,
  DbFilterMenu,
  DbSheet,
  DbSkeleton,
} from "~/components/ui";
import {
  ChevronRightIcon,
  HistoryIcon,
  RefreshIcon,
  ShieldIcon,
  TerminalIcon,
} from "~/assets/icons";

/**
 * Audit log: the append-only record of every action. Filter chips narrow
 * it by action, actor and place, and stay in the address. A row opens the
 * event in a drawer, with the hash chain's verdict on it. On phones the
 * rows are cards.
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
  scopeDescription,
  filters,
  actionOptions,
  actorOptions,
  whereOptions,
  whereValue,
  where,
  actorName,
  credential,
  formatDateTime,
  formatRelativeTime,
  canVerify,
  verifying,
  verifyError,
  chainSummary,
  selected,
} = controller;

/** Failures stand out; everything else reads as identity. */
function actionTone(action: string): "crit" | "accent" {
  return /(failed|denied)$/.test(action) ? "crit" : "accent";
}

const chainTone = {
  ok: "bg-ok/15 text-ink-strong [&>svg]:text-ok",
  crit: "bg-crit/15 text-ink-strong [&>svg]:text-crit",
  neutral: "bg-raised text-ink-muted",
};
</script>

<template>
  <DashboardLayout>
    <div class="page-width flex flex-col gap-5 py-6 lg:py-8">
      <PageHeader title="Audit log" :subtitle="scopeDescription">
        <template #actions>
          <DbButton
            variant="ghost"
            :loading="loading"
            aria-label="Refresh"
            @click="controller.load()">
            <RefreshIcon class="h-4 w-4" />
            <span class="hidden sm:inline">Refresh</span>
          </DbButton>
          <DbButton
            v-if="canVerify"
            variant="primary"
            :loading="verifying"
            data-testid="verify-chain"
            @click="controller.verifyChain()">
            <ShieldIcon class="h-4 w-4" />
            Verify chain
          </DbButton>
        </template>
      </PageHeader>

      <DbAlert
        v-if="chainSummary"
        :tone="chainSummary.tone === 'ok' ? 'success' : 'error'"
        data-testid="chain-summary">
        {{ chainSummary.text }}
      </DbAlert>
      <DbAlert v-else-if="verifyError">{{ verifyError }}</DbAlert>

      <div
        class="flex flex-wrap items-center gap-2"
        data-testid="audit-filters">
        <DbFilterMenu
          label="Action"
          :options="actionOptions"
          :model-value="filters.action || null"
          @update:model-value="controller.setAction" />
        <DbFilterMenu
          label="Actor"
          :options="actorOptions"
          :model-value="filters.actor || null"
          @update:model-value="controller.setActor" />
        <DbFilterMenu
          label="Where"
          :options="whereOptions"
          :model-value="whereValue"
          @update:model-value="controller.setWhere" />
      </div>

      <DbAlert v-if="loadError">{{ loadError }}</DbAlert>

      <div
        v-if="loading && !hasLoaded"
        class="flex flex-col gap-px overflow-hidden rounded-card border border-line bg-panel"
        data-testid="audit-skeleton">
        <div
          v-for="i in 6"
          :key="i"
          class="flex h-[52px] items-center gap-4 border-b border-line-soft px-4 last:border-b-0">
          <DbSkeleton class="h-4 w-16" />
          <DbSkeleton class="h-5 w-32 rounded-control" />
          <DbSkeleton class="hidden h-4 w-40 sm:block" />
        </div>
      </div>

      <DbEmptyState
        v-else-if="hasLoaded && items.length === 0"
        title="No audit events"
        description="Nothing matches these filters. Events show up here as projects, secrets, tokens and sign-ins change.">
        <template #icon>
          <HistoryIcon class="h-5 w-5" />
        </template>
      </DbEmptyState>

      <template v-else-if="items.length > 0">
        <!-- Phones: one card per event. -->
        <ul
          class="flex flex-col overflow-hidden rounded-card border border-line bg-panel sm:hidden"
          data-testid="audit-cards">
          <li
            v-for="event in items"
            :key="event.id"
            class="border-b border-line-soft last:border-b-0">
            <button
              type="button"
              class="flex w-full cursor-pointer items-center gap-3 px-4 py-3 text-left transition-colors hover:bg-raised/60"
              @click="controller.openEvent(event)">
              <span class="flex min-w-0 flex-1 flex-col gap-1">
                <span class="flex items-center justify-between gap-2">
                  <DbBadge :tone="actionTone(event.action)" class="truncate">
                    {{ event.action }}
                  </DbBadge>
                  <span
                    class="shrink-0 text-xs text-ink-muted"
                    :title="formatDateTime(event.createdAt)">
                    {{ formatRelativeTime(event.createdAt) }}
                  </span>
                </span>
                <span class="truncate font-mono text-xs text-ink">
                  {{ actorName(event) }}
                  <template v-if="where(event)"> · {{ where(event) }}</template>
                </span>
              </span>
              <ChevronRightIcon class="h-4 w-4 shrink-0 text-ink-faint" />
            </button>
          </li>
        </ul>

        <div
          class="hidden overflow-x-auto rounded-card border border-line bg-panel sm:block">
          <table
            class="min-w-full table-fixed text-left text-sm"
            data-testid="audit-table">
            <thead>
              <tr class="border-b border-line-soft">
                <th class="h-10 w-28 px-4 font-medium text-ink-muted">Time</th>
                <th class="h-10 w-56 px-4 font-medium text-ink-muted">
                  Action
                </th>
                <th class="h-10 px-4 font-medium text-ink-muted">Actor</th>
                <th class="h-10 w-44 px-4 font-medium text-ink-muted">Where</th>
                <th class="h-10 w-10 px-2">
                  <span class="sr-only">Open</span>
                </th>
              </tr>
            </thead>
            <tbody>
              <tr
                v-for="event in items"
                :key="event.id"
                class="cursor-pointer border-b border-line-soft transition-colors last:border-b-0 hover:bg-raised/60 focus-visible:bg-raised/60 focus-visible:outline-none"
                :class="selected?.id === event.id && 'bg-raised'"
                tabindex="0"
                data-testid="audit-row"
                @click="controller.openEvent(event)"
                @keydown.enter="controller.openEvent(event)">
                <td
                  class="h-[52px] whitespace-nowrap px-4 text-ink"
                  :title="formatDateTime(event.createdAt)">
                  {{ formatRelativeTime(event.createdAt) }}
                </td>
                <td class="px-4">
                  <DbBadge
                    :tone="actionTone(event.action)"
                    class="max-w-full truncate">
                    {{ event.action }}
                  </DbBadge>
                </td>
                <td class="px-4">
                  <span class="flex min-w-0 items-center gap-2">
                    <span class="truncate font-mono text-ink">
                      {{ actorName(event) }}
                    </span>
                    <DbBadge v-if="credential(event)" class="shrink-0">
                      via {{ credential(event) }}
                    </DbBadge>
                  </span>
                </td>
                <td class="truncate px-4 font-mono text-ink">
                  {{ where(event) }}
                </td>
                <td class="px-2">
                  <ChevronRightIcon class="h-4 w-4 text-ink-faint" />
                </td>
              </tr>
            </tbody>
          </table>
        </div>
      </template>

      <div v-if="nextCursor" class="flex justify-center">
        <DbButton
          variant="ghost"
          :loading="loadingMore"
          @click="controller.loadMore()">
          Load more
        </DbButton>
      </div>
    </div>

    <DbSheet
      :open="selected !== null"
      :title="selected?.action ?? ''"
      side="right"
      @close="controller.closeEvent">
      <div
        v-if="selected"
        class="flex min-h-0 flex-1 flex-col gap-3.5 overflow-y-auto px-5 py-4 text-sm"
        data-testid="audit-event">
        <p class="text-ink-muted">
          Event
          {{ selected.seq != null ? `#${selected.seq}` : selected.id }}
          ·
          <span :title="formatDateTime(selected.createdAt)">
            {{ formatRelativeTime(selected.createdAt) }}
          </span>
        </p>
        <dl class="flex flex-col gap-3.5">
          <div class="flex gap-3">
            <dt class="w-24 shrink-0 text-ink-muted sm:w-32">Actor</dt>
            <dd class="min-w-0 break-all font-mono text-ink-strong">
              {{ actorName(selected) }}
            </dd>
          </div>
          <div v-if="credential(selected)" class="flex gap-3">
            <dt class="w-24 shrink-0 text-ink-muted sm:w-32">Credential</dt>
            <dd>
              <DbBadge>{{ credential(selected) }}</DbBadge>
            </dd>
          </div>
          <div v-if="where(selected)" class="flex gap-3">
            <dt class="w-24 shrink-0 text-ink-muted sm:w-32">Where</dt>
            <dd class="min-w-0 break-all font-mono text-ink-strong">
              {{ where(selected) }}
            </dd>
          </div>
          <div
            v-if="selected.resourceType === 'secret' && selected.resourceId"
            class="flex gap-3">
            <dt class="w-24 shrink-0 text-ink-muted sm:w-32">Secret</dt>
            <dd class="min-w-0 break-all font-mono text-ink-strong">
              {{ selected.resourceId }}
            </dd>
          </div>
          <div v-else-if="selected.resourceType" class="flex gap-3">
            <dt class="w-24 shrink-0 text-ink-muted sm:w-32">Resource</dt>
            <dd class="min-w-0 break-all font-mono text-ink-strong">
              {{ selected.resourceType }}
              <template v-if="selected.resourceId">
                {{ selected.resourceId }}
              </template>
            </dd>
          </div>
          <div v-if="controller.origin(selected)" class="flex gap-3">
            <dt class="w-24 shrink-0 text-ink-muted sm:w-32">From</dt>
            <dd class="min-w-0 break-all font-mono text-ink-strong">
              {{ controller.origin(selected) }}
            </dd>
          </div>
        </dl>

        <div class="rounded-card border border-line bg-editor">
          <div
            class="flex items-center gap-2 border-b border-line-soft py-1 pl-3 pr-1">
            <TerminalIcon class="h-3.5 w-3.5 text-ink-muted" />
            <p class="flex-1 font-medium text-ink-muted">Event</p>
            <DbCopyButton
              :value="controller.eventPayload(selected)"
              label="Copy event"
              icon-only />
          </div>
          <pre
            class="overflow-x-auto p-3 font-mono text-xs leading-relaxed text-ink-strong"
            data-testid="audit-event-payload"
            >{{ controller.eventPayload(selected) }}</pre>
        </div>

        <p
          v-if="controller.chainLine(selected)"
          class="flex items-start gap-2.5 rounded-control px-3 py-2.5 font-mono text-xs leading-5"
          :class="chainTone[controller.chainLine(selected)!.tone]"
          data-testid="audit-chain">
          <ShieldIcon class="mt-0.5 h-4 w-4 shrink-0" />
          <span class="min-w-0 break-words">
            {{ controller.chainLine(selected)!.text }}
          </span>
        </p>
      </div>
    </DbSheet>
  </DashboardLayout>
</template>
