<script setup lang="ts">
import { useUsersController, STATUS_FILTERS } from "./Users.controller";
import UserDialog from "./components/UserDialog.vue";
import { SIGN_IN_LABELS } from "./people";
import type { SignInMethod } from "./people";
import { DashboardLayout } from "~/layouts";
import PageHeader from "~/components/app/PageHeader.vue";
import {
  DbAlert,
  DbAvatar,
  DbBadge,
  DbButton,
  DbConfirmDialog,
  DbFilterMenu,
  DbMenu,
  DbMenuItem,
  DbSearchField,
  DbSkeleton,
} from "~/components/ui";
import {
  LockIcon,
  LogOutIcon,
  PencilIcon,
  PlusIcon,
  ShieldIcon,
  TrashIcon,
} from "~/assets/icons";

const controller = useUsersController();
const {
  users,
  roles,
  ssoEnabled,
  loadError,
  canManage,
  filter,
  roleFilter,
  statusFilter,
  roleFilters,
  visibleRows,
  dialogOpen,
  editing,
  actions,
} = controller;
const { pending, copy, busy, error: actionError } = actions;

const signInClass: Record<SignInMethod, string> = {
  password: "bg-raised text-ink",
  sso: "bg-accent-soft text-accent-strong",
  disabled: "bg-crit/15 text-crit",
  none: "bg-raised text-ink-muted",
};
</script>

<template>
  <DashboardLayout>
    <div class="page-width flex flex-col gap-5 py-6 lg:py-8">
      <PageHeader
        title="Users"
        subtitle="People who sign in to this instance. Bots and CI live under AI agents.">
        <template v-if="canManage" #actions>
          <DbButton
            variant="primary"
            data-testid="add-user"
            @click="controller.openAdd">
            <PlusIcon class="h-4 w-4" />
            Add user
          </DbButton>
        </template>
      </PageHeader>

      <div class="flex flex-wrap items-center gap-2">
        <DbSearchField
          v-model="filter"
          class="w-full sm:w-64"
          placeholder="Filter by email" />
        <DbFilterMenu
          v-model="roleFilter"
          label="Role"
          :options="roleFilters" />
        <DbFilterMenu
          v-model="statusFilter"
          label="Status"
          :options="STATUS_FILTERS" />
      </div>

      <div v-if="loadError" class="flex flex-col items-start gap-3">
        <DbAlert>{{ loadError }}</DbAlert>
        <DbButton size="sm" @click="controller.load">Try again</DbButton>
      </div>

      <div v-else-if="users === null" class="flex flex-col gap-2">
        <DbSkeleton
          v-for="line in 5"
          :key="line"
          class="h-12 w-full rounded-control" />
      </div>

      <template v-else>
        <!-- Table on wider screens -->
        <div
          class="hidden overflow-x-auto rounded-card border border-line bg-panel sm:block">
          <table class="min-w-full text-left text-sm" data-testid="users-table">
            <thead>
              <tr class="border-b border-line-soft bg-raised/50">
                <th class="h-11 px-4 font-medium text-ink-muted">Person</th>
                <th class="h-11 px-4 font-medium text-ink-muted">Role</th>
                <th class="h-11 px-4 font-medium text-ink-muted">Groups</th>
                <th class="h-11 px-4 font-medium text-ink-muted">Sign-in</th>
                <th class="h-11 px-4 font-medium text-ink-muted">
                  Last sign-in
                </th>
                <th class="h-11 w-12 px-2">
                  <span class="sr-only">Actions</span>
                </th>
              </tr>
            </thead>
            <tbody>
              <tr
                v-for="row in visibleRows"
                :key="row.user.id"
                class="border-b border-line-soft last:border-b-0"
                data-testid="user-row">
                <td class="h-[52px] px-4">
                  <RouterLink
                    :to="{ name: 'user', params: { userId: row.user.id } }"
                    class="flex min-w-0 items-center gap-2.5 hover:underline">
                    <DbAvatar
                      :name="row.user.email"
                      :muted="row.user.status === 'disabled'" />
                    <span
                      class="truncate font-mono"
                      :class="
                        row.user.status === 'disabled'
                          ? 'text-ink-faint'
                          : 'text-ink-strong'
                      ">
                      {{ row.user.email }}
                    </span>
                    <span v-if="row.isYou" class="text-xs text-ink-faint">
                      you
                    </span>
                  </RouterLink>
                </td>
                <td class="px-4">
                  <span class="inline-flex items-center gap-1.5">
                    <DbBadge :tone="row.isRoot ? 'accent' : 'neutral'">
                      {{ row.roleName }}
                    </DbBadge>
                    <span
                      v-if="row.customRole"
                      class="font-mono text-xs text-ink-faint">
                      custom
                    </span>
                  </span>
                </td>
                <td class="px-4">
                  <span
                    v-if="row.user.groups.length === 0"
                    class="text-ink-faint">
                    —
                  </span>
                  <span v-else class="flex flex-wrap gap-x-2">
                    <RouterLink
                      v-for="group in row.user.groups"
                      :key="group.id"
                      :to="{ name: 'group', params: { groupId: group.id } }"
                      class="text-accent-strong hover:underline">
                      {{ group.name }}
                    </RouterLink>
                  </span>
                </td>
                <td class="px-4">
                  <span
                    class="rounded-control px-1.5 py-0.5 font-mono text-xs"
                    :class="signInClass[row.signIn]">
                    {{ SIGN_IN_LABELS[row.signIn] }}
                  </span>
                </td>
                <td class="px-4 text-ink-muted">{{ row.lastSignIn }}</td>
                <td class="px-2 text-right">
                  <DbMenu
                    v-if="row.canEdit"
                    :label="`Actions for ${row.user.email}`">
                    <DbMenuItem @select="controller.openEdit(row.user)">
                      <template #icon><PencilIcon class="h-4 w-4" /></template>
                      Edit role and email
                    </DbMenuItem>
                    <DbMenuItem
                      @select="
                        $router.push({
                          name: 'user-access',
                          params: { userId: row.user.id },
                        })
                      ">
                      <template #icon><ShieldIcon class="h-4 w-4" /></template>
                      Access
                    </DbMenuItem>
                    <DbMenuItem
                      v-if="row.actions.includes('unlink')"
                      @select="actions.ask('unlink', row.user)">
                      <template #icon><LogOutIcon class="h-4 w-4" /></template>
                      Unlink single sign-on
                    </DbMenuItem>
                    <DbMenuItem
                      v-if="row.actions.includes('disable')"
                      @select="actions.ask('disable', row.user)">
                      <template #icon><LockIcon class="h-4 w-4" /></template>
                      Disable account
                    </DbMenuItem>
                    <DbMenuItem
                      v-if="row.actions.includes('enable')"
                      @select="actions.ask('enable', row.user)">
                      <template #icon><LockIcon class="h-4 w-4" /></template>
                      Turn account back on
                    </DbMenuItem>
                    <div class="my-0.5 h-px bg-line-soft" role="separator" />
                    <DbMenuItem
                      tone="danger"
                      @select="actions.ask('delete', row.user)">
                      <template #icon><TrashIcon class="h-4 w-4" /></template>
                      Delete user
                    </DbMenuItem>
                  </DbMenu>
                </td>
              </tr>
            </tbody>
          </table>
          <p
            v-if="visibleRows.length === 0"
            class="px-4 py-6 text-center text-sm text-ink-muted">
            No users match these filters.
          </p>
        </div>

        <!-- Cards on phones -->
        <ul class="flex flex-col gap-2 sm:hidden" data-testid="users-cards">
          <li
            v-for="row in visibleRows"
            :key="row.user.id"
            class="flex items-start gap-3 rounded-card border border-line bg-panel p-3">
            <DbAvatar
              :name="row.user.email"
              :muted="row.user.status === 'disabled'" />
            <RouterLink
              :to="{ name: 'user', params: { userId: row.user.id } }"
              class="flex min-w-0 flex-1 flex-col gap-1">
              <span
                class="font-mono text-sm text-ink-strong [overflow-wrap:anywhere]">
                {{ row.user.email }}
              </span>
              <span class="flex flex-wrap items-center gap-1.5 text-xs">
                <DbBadge :tone="row.isRoot ? 'accent' : 'neutral'">
                  {{ row.roleName }}
                </DbBadge>
                <span
                  class="rounded-control px-1.5 py-0.5 font-mono"
                  :class="signInClass[row.signIn]">
                  {{ SIGN_IN_LABELS[row.signIn] }}
                </span>
                <span class="text-ink-muted">{{ row.lastSignIn }}</span>
              </span>
            </RouterLink>
          </li>
          <li
            v-if="visibleRows.length === 0"
            class="py-6 text-center text-sm text-ink-muted">
            No users match these filters.
          </li>
        </ul>
      </template>
    </div>

    <UserDialog
      :open="dialogOpen"
      :roles="roles"
      :sso-enabled="ssoEnabled"
      :editing="editing"
      @close="dialogOpen = false"
      @saved="controller.saved" />

    <DbConfirmDialog
      :open="pending !== null"
      :title="copy?.title ?? ''"
      :description="copy?.description"
      :confirm-label="copy?.confirmLabel"
      :tone="copy?.tone"
      :confirm-word="copy?.confirmWord"
      :loading="busy"
      :error="actionError"
      @confirm="actions.confirm"
      @close="actions.cancel" />
  </DashboardLayout>
</template>
