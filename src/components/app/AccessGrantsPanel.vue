<script setup lang="ts">
import { toRef } from "vue";
import type { RouteLocationRaw } from "vue-router";
import { useAccessGrantsPanel } from "./AccessGrantsPanel.controller";
import ChangeAccessDialog from "./ChangeAccessDialog.vue";
import {
  DbAlert,
  DbButton,
  DbCheckbox,
  DbConfirmDialog,
  DbInfoTip,
  DbLevelBadge,
  DbMenu,
  DbMenuItem,
  DbSearchField,
} from "~/components/ui";
import {
  ChevronRightIcon,
  FolderIcon,
  LockIcon,
  PlusIcon,
  SlidersHorizontalIcon,
  TrashIcon,
  UsersIcon,
  XIcon,
} from "~/assets/icons";
import type { Grant } from "~/services/access.api";
import type { GrantPrincipal } from "~/utils/level";

/**
 * AccessGrantsPanel — the Access tab for a user, AI agent or group: what
 * they reach, where it comes from, and Give access, which opens its own
 * page. Direct grants have a checkbox, so several can be removed at once.
 * The page passes the grants and reloads them when `changed` fires.
 */
const props = withDefaults(
  defineProps<{
    principal: GrantPrincipal;
    grants: Grant[];
    /** Where Give access goes: the Give access page for this principal. */
    giveTo: RouteLocationRaw;
    /** The info line above the table. */
    summary?: string;
    /** Hide Give access, change and remove, for read-only viewers. */
    readOnly?: boolean;
  }>(),
  { summary: "", readOnly: false },
);

const emit = defineEmits<{ changed: [] }>();

const panel = useAccessGrantsPanel(
  toRef(props, "principal"),
  toRef(props, "grants"),
  () => emit("changed"),
  toRef(props, "readOnly"),
);
const {
  filter,
  rows,
  sourceHint,
  visibleRows,
  overrides,
  editing,
  canSelect,
  selectedRows,
  allVisibleSelected,
  someVisibleSelected,
  removing,
  removeTitle,
  removeDescription,
  removeBusy,
  removeError,
} = panel;
</script>

<template>
  <div class="flex flex-col gap-4" data-testid="access-grants">
    <DbAlert v-if="summary" tone="info">{{ summary }}</DbAlert>

    <div class="flex flex-wrap items-center gap-2">
      <DbSearchField
        v-model="filter"
        class="w-full sm:w-64"
        placeholder="Filter projects" />
      <div
        v-if="selectedRows.length > 0"
        class="flex items-center gap-2 sm:ml-auto"
        data-testid="selection-bar">
        <span class="text-sm text-ink-muted" aria-live="polite">
          {{ selectedRows.length }} selected
        </span>
        <DbButton variant="ghost" @click="panel.clearSelection">
          Clear
        </DbButton>
        <DbButton
          variant="danger"
          data-testid="remove-selected"
          @click="panel.askRemoveSelected">
          <TrashIcon class="h-4 w-4" />
          Remove access
        </DbButton>
      </div>
      <DbButton
        v-else-if="!readOnly"
        variant="primary"
        class="sm:ml-auto"
        data-testid="give-access"
        @click="$router.push(giveTo)">
        <PlusIcon class="h-4 w-4" />
        Give access
      </DbButton>
    </div>

    <div
      v-if="rows.length === 0"
      class="rounded-card border border-dashed border-line px-6 py-10 text-center text-sm text-ink-muted">
      {{ principal.label }} doesn't reach any project yet.
    </div>
    <div
      v-else
      class="overflow-x-auto rounded-card border border-line bg-panel">
      <table class="min-w-full text-left text-sm" data-testid="grants-table">
        <thead>
          <tr class="border-b border-line-soft bg-raised/50">
            <th v-if="canSelect" class="h-11 w-10 pl-4 pr-0">
              <DbCheckbox
                :model-value="allVisibleSelected"
                :indeterminate="someVisibleSelected"
                aria-label="Select all grants you can remove"
                data-testid="select-all"
                @update:model-value="panel.toggleAllVisible" />
            </th>
            <th class="h-11 px-4 font-medium text-ink-muted">
              Project / environment
            </th>
            <th class="h-11 px-4 font-medium text-ink-muted">
              <span class="inline-flex items-center gap-1.5">
                Source
                <DbInfoTip :text="sourceHint" label="About Source" />
              </span>
            </th>
            <th class="h-11 px-4 font-medium text-ink-muted">Level</th>
            <th class="h-11 w-12 px-2">
              <span class="sr-only">Actions</span>
            </th>
          </tr>
        </thead>
        <tbody>
          <tr
            v-for="row in visibleRows"
            :key="row.grant.id"
            class="border-b border-line-soft transition-colors last:border-b-0"
            :class="panel.isSelected(row) ? 'bg-accent-soft/40' : ''"
            data-testid="grant-row">
            <td v-if="canSelect" class="w-10 pl-4 pr-0">
              <DbCheckbox
                v-if="row.editable"
                :model-value="panel.isSelected(row)"
                :aria-label="`Select ${row.environment ? `${row.project}/${row.environment}` : row.project}`"
                data-testid="select-grant"
                @update:model-value="panel.toggleRow(row, $event)" />
            </td>
            <td class="h-[52px] px-4">
              <span class="inline-flex flex-wrap items-center gap-2">
                <span class="font-mono text-ink-strong">{{ row.project }}</span>
                <span
                  v-if="row.environment"
                  class="rounded-control border border-line px-1.5 font-mono text-[13px] text-ink">
                  {{ row.environment }}
                </span>
                <span v-else-if="!row.allProjects" class="text-ink-muted">
                  whole project
                </span>
              </span>
            </td>
            <td class="px-4">
              <RouterLink
                v-if="row.viaGroup"
                :to="{ name: 'group', params: { groupId: row.viaGroup.id } }"
                class="inline-flex items-center gap-1.5 text-accent-strong hover:underline">
                <UsersIcon class="h-4 w-4" />
                via {{ row.viaGroup.name }}
              </RouterLink>
              <span v-else class="text-ink-muted">Direct</span>
            </td>
            <td class="px-4"><DbLevelBadge v-bind="row.meter" /></td>
            <td class="px-2 text-right">
              <DbMenu
                v-if="row.hasActions"
                :label="`Actions for ${row.project}`">
                <DbMenuItem
                  v-if="row.editable"
                  @select="panel.openChange(row.grant)">
                  <template #icon>
                    <SlidersHorizontalIcon class="h-4 w-4" />
                  </template>
                  Change level
                </DbMenuItem>
                <DbMenuItem
                  v-if="row.viaGroup"
                  @select="
                    $router.push({
                      name: 'group',
                      params: { groupId: row.viaGroup.id },
                    })
                  ">
                  <template #icon><UsersIcon class="h-4 w-4" /></template>
                  Open group
                </DbMenuItem>
                <DbMenuItem
                  v-if="row.projectLink"
                  @select="$router.push(row.projectLink)">
                  <template #icon><FolderIcon class="h-4 w-4" /></template>
                  Open project
                </DbMenuItem>
                <template v-if="row.editable">
                  <div class="my-0.5 h-px bg-line-soft" role="separator" />
                  <DbMenuItem tone="danger" @select="panel.askRemove(row)">
                    <template #icon><XIcon class="h-4 w-4" /></template>
                    Remove access
                  </DbMenuItem>
                </template>
              </DbMenu>
            </td>
          </tr>
        </tbody>
      </table>
      <p
        v-if="visibleRows.length === 0"
        class="px-4 py-6 text-center text-sm text-ink-muted">
        No projects match "{{ filter.trim() }}".
      </p>
    </div>

    <DbAlert
      v-if="overrides.length > 0"
      tone="info"
      title="Environment grants come first"
      data-testid="grant-overrides">
      <p>
        In these environments, {{ principal.label }} gets the environment's
        level instead of the project's.
      </p>
      <ul class="mt-2.5 flex flex-col gap-2">
        <li
          v-for="item in overrides"
          :key="item.key"
          class="flex flex-wrap items-center gap-x-2 gap-y-1.5"
          data-testid="grant-override">
          <span class="inline-flex min-w-0 items-center gap-1.5">
            <span class="font-mono text-ink-strong">{{ item.project }}</span>
            <span
              class="rounded-control border border-line px-1.5 font-mono text-[13px] text-ink">
              {{ item.environment }}
            </span>
          </span>
          <span class="inline-flex items-center gap-1.5 sm:ml-auto">
            <DbLevelBadge
              v-bind="item.from"
              class="opacity-60"
              :aria-label="`Project grant: ${item.from.label}`" />
            <ChevronRightIcon
              class="h-4 w-4 shrink-0 text-ink-muted"
              aria-hidden="true" />
            <DbLevelBadge
              v-if="item.to"
              v-bind="item.to"
              :aria-label="`In ${item.environment}: ${item.to.label}`" />
            <span
              v-else
              class="inline-flex items-center gap-1.5 whitespace-nowrap rounded-control bg-crit/15 py-0.5 pl-2 pr-2.5 text-sm font-medium leading-5 text-crit">
              <LockIcon class="h-3.5 w-3.5" aria-hidden="true" />
              No access
            </span>
          </span>
        </li>
      </ul>
    </DbAlert>

    <ChangeAccessDialog
      :principal="principal"
      :grant="editing"
      @close="editing = null"
      @saved="panel.saved" />

    <DbConfirmDialog
      :open="removing !== null"
      :title="removeTitle"
      :description="removeDescription"
      confirm-label="Remove access"
      :loading="removeBusy"
      :error="removeError"
      @confirm="panel.confirmRemove"
      @close="panel.cancelRemove" />
  </div>
</template>
