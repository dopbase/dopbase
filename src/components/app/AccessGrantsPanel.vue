<script setup lang="ts">
import { computed, toRef } from "vue";
import { useAccessGrantsPanel } from "./AccessGrantsPanel.controller";
import GiveAccessDialog from "./GiveAccessDialog.vue";
import type { GrantPrincipal } from "./GiveAccessDialog.controller";
import {
  DbAlert,
  DbButton,
  DbConfirmDialog,
  DbLevelBadge,
  DbMenu,
  DbMenuItem,
  DbSearchField,
} from "~/components/ui";
import {
  FolderIcon,
  PlusIcon,
  SlidersHorizontalIcon,
  UsersIcon,
  XIcon,
} from "~/assets/icons";
import type { Grant } from "~/services/access.api";

/**
 * AccessGrantsPanel — the Access tab for a user, AI agent or group: what
 * they reach, where it comes from, and Give access. The page passes the
 * grants and reloads them when `changed` fires.
 */
const props = withDefaults(
  defineProps<{
    principal: GrantPrincipal;
    grants: Grant[];
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
);
const {
  filter,
  rows,
  visibleRows,
  overrides,
  dialogOpen,
  editing,
  removing,
  removeBusy,
  removeError,
} = panel;

const directGrants = computed(() =>
  props.grants.filter(
    (grant) =>
      grant.principalType === props.principal.type &&
      grant.principalId === props.principal.id,
  ),
);
</script>

<template>
  <div class="flex flex-col gap-4" data-testid="access-grants">
    <DbAlert v-if="summary" tone="info">{{ summary }}</DbAlert>

    <div class="flex flex-wrap items-center gap-2">
      <DbSearchField
        v-model="filter"
        class="w-full sm:w-64"
        placeholder="Filter projects" />
      <DbButton
        v-if="!readOnly"
        variant="primary"
        class="sm:ml-auto"
        data-testid="give-access"
        @click="panel.openGive">
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
            <th class="h-11 px-4 font-medium text-ink-muted">
              Project / environment
            </th>
            <th class="h-11 px-4 font-medium text-ink-muted">Source</th>
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
            class="border-b border-line-soft last:border-b-0"
            data-testid="grant-row">
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
              <DbMenu :label="`Actions for ${row.project}`">
                <DbMenuItem
                  v-if="!readOnly && !row.viaGroup && !row.allProjects"
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
                <template v-if="!readOnly && !row.viaGroup && !row.allProjects">
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

    <p v-for="line in overrides" :key="line" class="text-[13px] text-ink-muted">
      {{ line }}
    </p>

    <GiveAccessDialog
      :open="dialogOpen"
      :principal="principal"
      :existing="directGrants"
      :editing="editing"
      @close="dialogOpen = false"
      @saved="panel.saved" />

    <DbConfirmDialog
      :open="removing !== null"
      :title="`Remove access to ${removing?.environment ? `${removing.project}/${removing.environment}` : removing?.project}?`"
      :description="`${principal.label} loses this access on their next request. Other grants they hold stay.`"
      confirm-label="Remove access"
      :loading="removeBusy"
      :error="removeError"
      @confirm="panel.confirmRemove"
      @close="panel.cancelRemove" />
  </div>
</template>
