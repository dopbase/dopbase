<script setup lang="ts">
import { useGroupController } from "./Group.controller";
import GroupDialog from "./components/GroupDialog.vue";
import AddMembersDialog from "./components/AddMembersDialog.vue";
import { DashboardLayout } from "~/layouts";
import PageHeader from "~/components/app/PageHeader.vue";
import AccessGrantsPanel from "~/components/app/AccessGrantsPanel.vue";
import {
  DbAlert,
  DbAvatar,
  DbButton,
  DbConfirmDialog,
  DbMenu,
  DbMenuItem,
  DbSkeleton,
  DbTabNav,
} from "~/components/ui";
import {
  InfoIcon,
  PencilIcon,
  PlusIcon,
  UserIcon,
  XIcon,
} from "~/assets/icons";

const controller = useGroupController();
const {
  activeTab,
  tabs,
  canManage,
  detail,
  group,
  people,
  loadError,
  subtitle,
  memberRows,
  membersError,
  principal,
  accessSummary,
  membersTarget,
  renameOpen,
  deleteOpen,
  deleteBusy,
  deleteError,
} = controller;
</script>

<template>
  <DashboardLayout>
    <div class="page-width flex flex-col gap-5 py-6 lg:py-8">
      <div v-if="loadError" class="flex flex-col items-start gap-3">
        <DbAlert>{{ loadError }}</DbAlert>
        <DbButton size="sm" @click="controller.load">Try again</DbButton>
      </div>

      <div v-else-if="!detail || !group" class="flex flex-col gap-3">
        <DbSkeleton class="h-8 w-72 rounded-control" />
        <DbSkeleton class="h-48 w-full rounded-card" />
      </div>

      <template v-else>
        <PageHeader
          :title="group.name"
          :subtitle="subtitle"
          :parent="{ label: 'Groups', to: { name: 'groups' } }">
          <template v-if="canManage" #actions>
            <DbButton data-testid="rename-group" @click="renameOpen = true">
              <PencilIcon class="h-4 w-4" />
              Rename
            </DbButton>
          </template>
        </PageHeader>

        <DbTabNav :tabs="tabs" :active="activeTab" label="Group sections" />

        <!-- Members -->
        <div
          v-if="activeTab === 'members'"
          class="flex flex-col gap-4"
          data-testid="group-members">
          <div class="flex flex-wrap items-start justify-between gap-3">
            <p class="flex max-w-2xl items-start gap-2 text-sm text-ink-muted">
              <InfoIcon class="mt-0.5 h-4 w-4 shrink-0" />
              Members get the group's access, but never more than their own role
              allows.
            </p>
            <DbButton
              v-if="canManage"
              variant="primary"
              data-testid="add-members"
              @click="controller.openMembers">
              <PlusIcon class="h-4 w-4" />
              Add members
            </DbButton>
          </div>
          <DbAlert v-if="membersError">{{ membersError }}</DbAlert>
          <div
            v-if="memberRows.length === 0"
            class="rounded-card border border-dashed border-line px-6 py-10 text-center text-sm text-ink-muted">
            Nobody is in {{ group.name }} yet.
          </div>
          <div
            v-else
            class="overflow-x-auto rounded-card border border-line bg-panel">
            <table class="min-w-full text-left text-sm">
              <thead>
                <tr class="border-b border-line-soft bg-raised/50">
                  <th class="h-11 px-4 font-medium text-ink-muted">Member</th>
                  <th class="h-11 px-4 font-medium text-ink-muted">Role</th>
                  <th class="h-11 px-4 font-medium text-ink-muted">Added</th>
                  <th class="h-11 w-12 px-2">
                    <span class="sr-only">Actions</span>
                  </th>
                </tr>
              </thead>
              <tbody>
                <tr
                  v-for="row in memberRows"
                  :key="row.member.id"
                  class="border-b border-line-soft last:border-b-0"
                  data-testid="member-row">
                  <td class="h-[52px] px-4">
                    <RouterLink
                      :to="row.to"
                      class="inline-flex items-center gap-2.5 font-mono text-ink-strong hover:underline">
                      <DbAvatar :name="row.member.email" size="sm" />
                      {{ row.member.email }}
                    </RouterLink>
                  </td>
                  <td class="px-4 text-ink">{{ row.role || "—" }}</td>
                  <td class="px-4 text-ink-muted">{{ row.added }}</td>
                  <td class="px-2 text-right">
                    <DbMenu :label="`Actions for ${row.member.email}`">
                      <DbMenuItem @select="$router.push(row.to)">
                        <template #icon><UserIcon class="h-4 w-4" /></template>
                        Open
                      </DbMenuItem>
                      <DbMenuItem
                        v-if="canManage"
                        tone="danger"
                        @select="controller.removeMember(row.member)">
                        <template #icon><XIcon class="h-4 w-4" /></template>
                        Remove from group
                      </DbMenuItem>
                    </DbMenu>
                  </td>
                </tr>
              </tbody>
            </table>
          </div>
        </div>

        <!-- Access -->
        <AccessGrantsPanel
          v-else-if="activeTab === 'access'"
          :principal="principal"
          :grants="detail.grants"
          :summary="accessSummary"
          :read-only="!canManage"
          @changed="controller.reload" />

        <!-- Danger zone -->
        <div
          v-else-if="activeTab === 'danger' && canManage"
          class="flex flex-wrap items-center justify-between gap-3 rounded-card border border-crit/60 p-4 sm:p-5"
          data-testid="group-danger">
          <div>
            <p class="text-sm font-medium text-ink-strong">Delete the group</p>
            <p class="text-sm text-ink-muted">
              Members lose the access it gives them. Their own grants stay.
            </p>
          </div>
          <DbButton
            variant="danger"
            data-testid="delete-group"
            @click="controller.askDelete">
            Delete group
          </DbButton>
        </div>
      </template>
    </div>

    <GroupDialog
      :open="renameOpen"
      :editing="group"
      @close="renameOpen = false"
      @saved="controller.renamed" />

    <AddMembersDialog
      :open="membersTarget !== null"
      :target="membersTarget"
      :people="people"
      @close="membersTarget = null"
      @saved="controller.membersAdded" />

    <DbConfirmDialog
      :open="deleteOpen"
      :title="`Delete ${group?.name ?? 'the group'}?`"
      :description="`Everyone in ${group?.name ?? 'the group'} loses the access it gives them. Their own grants stay.`"
      :confirm-word="group?.name"
      confirm-label="Delete group"
      :loading="deleteBusy"
      :error="deleteError"
      @confirm="controller.confirmDelete"
      @close="controller.cancelDelete" />
  </DashboardLayout>
</template>
