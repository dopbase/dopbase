<script setup lang="ts">
import { useGroupsController } from "./Groups.controller";
import GroupDialog from "./components/GroupDialog.vue";
import AddMembersDialog from "./components/AddMembersDialog.vue";
import { DashboardLayout } from "~/layouts";
import PageHeader from "~/components/app/PageHeader.vue";
import GiveAccessDialog from "~/components/app/GiveAccessDialog.vue";
import {
  DbAlert,
  DbAvatar,
  DbButton,
  DbConfirmDialog,
  DbMenu,
  DbMenuItem,
  DbSearchField,
  DbSkeleton,
} from "~/components/ui";
import {
  FolderIcon,
  PencilIcon,
  PlusIcon,
  TrashIcon,
  UsersIcon,
} from "~/assets/icons";

/**
 * Groups lists every group with who's in it and where it reaches. Each
 * group's own page holds its members, access and Danger zone.
 */
const controller = useGroupsController();
const {
  groups,
  people,
  loadError,
  filter,
  canManage,
  visibleRows,
  dialogOpen,
  editing,
  membersTarget,
  accessFor,
  accessPrincipal,
  deleting,
  deleteBusy,
  deleteError,
} = controller;
</script>

<template>
  <DashboardLayout>
    <div class="page-width flex flex-col gap-5 py-6 lg:py-8">
      <PageHeader
        title="Groups"
        subtitle="Give a group access once, and everyone in it gets it. Leaving the group takes it away on their next request.">
        <template v-if="canManage" #actions>
          <DbButton
            variant="primary"
            data-testid="new-group"
            @click="controller.openNew">
            <PlusIcon class="h-4 w-4" />
            New group
          </DbButton>
        </template>
      </PageHeader>

      <DbSearchField
        v-model="filter"
        class="w-full sm:w-64"
        placeholder="Filter groups" />

      <DbAlert v-if="loadError">{{ loadError }}</DbAlert>

      <div v-if="groups === null" class="flex flex-col gap-2">
        <DbSkeleton
          v-for="line in 3"
          :key="line"
          class="h-12 w-full rounded-control" />
      </div>

      <div
        v-else-if="groups.length === 0"
        class="rounded-card border border-dashed border-line px-6 py-10 text-center">
        <p class="text-sm font-medium text-ink-strong">No groups yet</p>
        <p class="mt-1 text-sm text-ink-muted">
          Groups save you giving the same access to people one at a time.
        </p>
      </div>

      <template v-else>
        <!-- Table on wider screens -->
        <div
          class="hidden overflow-x-auto rounded-card border border-line bg-panel sm:block">
          <table
            class="min-w-full text-left text-sm"
            data-testid="groups-table">
            <thead>
              <tr class="border-b border-line-soft bg-raised/50">
                <th class="h-11 px-4 font-medium text-ink-muted">Group</th>
                <th class="h-11 px-4 font-medium text-ink-muted">Members</th>
                <th class="h-11 px-4 font-medium text-ink-muted">Access</th>
                <th class="h-11 px-4 font-medium text-ink-muted">Created by</th>
                <th class="h-11 w-12 px-2">
                  <span class="sr-only">Actions</span>
                </th>
              </tr>
            </thead>
            <tbody>
              <tr
                v-for="row in visibleRows"
                :key="row.group.id"
                class="border-b border-line-soft last:border-b-0"
                data-testid="group-row">
                <td class="h-[60px] max-w-72 px-4">
                  <RouterLink
                    :to="{ name: 'group', params: { groupId: row.group.id } }"
                    class="block truncate font-medium text-ink-strong hover:underline">
                    {{ row.group.name }}
                  </RouterLink>
                  <p
                    v-if="row.group.description"
                    class="truncate text-xs text-ink-muted">
                    {{ row.group.description }}
                  </p>
                </td>
                <td class="px-4">
                  <span
                    v-if="row.preview.length"
                    class="inline-flex items-center gap-1"
                    :title="row.memberLabel">
                    <DbAvatar
                      v-for="member in row.preview"
                      :key="member.id"
                      :name="member.email"
                      size="sm" />
                    <span
                      v-if="row.more"
                      class="ml-1 font-mono text-xs text-ink-muted">
                      +{{ row.more }}
                    </span>
                    <span class="sr-only">{{ row.memberLabel }}</span>
                  </span>
                  <span v-else class="text-ink-muted">{{
                    row.memberLabel
                  }}</span>
                </td>
                <td class="px-4">
                  <span
                    v-if="row.reach.length"
                    class="flex flex-wrap items-center gap-1.5">
                    <span
                      v-for="(place, index) in row.reach"
                      :key="index"
                      class="rounded-control bg-raised px-1.5 py-0.5 font-mono text-xs text-ink">
                      {{ place }}
                    </span>
                  </span>
                  <span v-else class="text-ink-muted">No access yet</span>
                </td>
                <td class="px-4 font-mono text-ink-muted">
                  {{ row.createdBy }}
                </td>
                <td class="px-2 text-right">
                  <DbMenu
                    v-if="canManage"
                    :label="`Actions for ${row.group.name}`">
                    <DbMenuItem @select="controller.openRename(row.group)">
                      <template #icon><PencilIcon class="h-4 w-4" /></template>
                      Rename
                    </DbMenuItem>
                    <DbMenuItem @select="controller.openMembers(row.group)">
                      <template #icon><UsersIcon class="h-4 w-4" /></template>
                      Add members
                    </DbMenuItem>
                    <DbMenuItem @select="accessFor = row.group">
                      <template #icon><FolderIcon class="h-4 w-4" /></template>
                      Give access
                    </DbMenuItem>
                    <div class="my-0.5 h-px bg-line-soft" role="separator" />
                    <DbMenuItem
                      tone="danger"
                      @select="controller.askDelete(row.group)">
                      <template #icon><TrashIcon class="h-4 w-4" /></template>
                      Delete group
                    </DbMenuItem>
                  </DbMenu>
                </td>
              </tr>
            </tbody>
          </table>
          <p
            v-if="visibleRows.length === 0"
            class="px-4 py-6 text-center text-sm text-ink-muted">
            No groups match that filter.
          </p>
        </div>

        <!-- Cards on phones -->
        <ul class="flex flex-col gap-2 sm:hidden" data-testid="groups-cards">
          <li
            v-for="row in visibleRows"
            :key="row.group.id"
            class="rounded-card border border-line bg-panel p-3">
            <RouterLink
              :to="{ name: 'group', params: { groupId: row.group.id } }"
              class="flex flex-col gap-1">
              <span class="truncate text-sm font-medium text-ink-strong">
                {{ row.group.name }}
              </span>
              <span
                v-if="row.group.description"
                class="truncate text-xs text-ink-muted">
                {{ row.group.description }}
              </span>
              <span class="text-xs text-ink-muted">
                {{ row.memberLabel }} ·
                {{ row.reach.length ? row.reach.join(", ") : "No access yet" }}
              </span>
            </RouterLink>
          </li>
        </ul>
      </template>
    </div>

    <GroupDialog
      :open="dialogOpen"
      :editing="editing"
      @close="dialogOpen = false"
      @saved="controller.saved" />

    <AddMembersDialog
      :open="membersTarget !== null"
      :target="membersTarget"
      :people="people"
      @close="membersTarget = null"
      @saved="controller.membersAdded" />

    <GiveAccessDialog
      v-if="accessPrincipal"
      :open="accessFor !== null"
      :principal="accessPrincipal"
      :existing="accessFor?.grants ?? []"
      @close="accessFor = null"
      @saved="controller.accessGiven" />

    <DbConfirmDialog
      :open="deleting !== null"
      :title="`Delete ${deleting?.name ?? 'the group'}?`"
      :description="`Everyone in ${deleting?.name ?? 'the group'} loses the access it gives them. Their own grants stay.`"
      :confirm-word="deleting?.name"
      confirm-label="Delete group"
      :loading="deleteBusy"
      :error="deleteError"
      @confirm="controller.confirmDelete"
      @close="controller.cancelDelete" />
  </DashboardLayout>
</template>
