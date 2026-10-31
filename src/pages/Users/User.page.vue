<script setup lang="ts">
import { useUserController, type TokenState } from "./User.controller";
import UserDialog from "./components/UserDialog.vue";
import { DashboardLayout } from "~/layouts";
import PageHeader from "~/components/app/PageHeader.vue";
import AccessGrantsPanel from "~/components/app/AccessGrantsPanel.vue";
import {
  DbAlert,
  DbBadge,
  DbButton,
  DbConfirmDialog,
  DbLevelBadge,
  DbMenu,
  DbMenuItem,
  DbSkeleton,
  DbTabNav,
} from "~/components/ui";
import { PencilIcon, PlusIcon, UsersIcon, XIcon } from "~/assets/icons";
import { formatLevel } from "~/utils/format";
import { levelMeter } from "~/utils/level";
import { SIGN_IN_LABELS, signInMethod } from "./people";

const controller = useUserController();
const {
  activeTab,
  tabs,
  user,
  access,
  roles,
  role,
  ssoEnabled,
  loadError,
  isRoot,
  canManage,
  subtitle,
  created,
  accessSummary,
  principal,
  canManageGroups,
  joinable,
  groupsError,
  tokens,
  tokenRows,
  tokensError,
  canRevokeTokens,
  editOpen,
  actions,
} = controller;
const { pending, copy, busy, error: actionError } = actions;

const stateClass: Record<TokenState, string> = {
  active: "bg-ok/15 text-ok",
  expired: "bg-raised text-ink-muted",
  revoked: "bg-crit/15 text-crit",
};
const card = "rounded-card border border-line bg-panel p-4 sm:p-5";
</script>

<template>
  <DashboardLayout>
    <div class="page-width flex flex-col gap-5 py-6 lg:py-8">
      <div v-if="loadError" class="flex flex-col items-start gap-3">
        <DbAlert>{{ loadError }}</DbAlert>
        <DbButton size="sm" @click="controller.load">Try again</DbButton>
      </div>

      <div v-else-if="!user" class="flex flex-col gap-3">
        <DbSkeleton class="h-8 w-72 rounded-control" />
        <DbSkeleton class="h-48 w-full rounded-card" />
      </div>

      <template v-else>
        <PageHeader
          :title="user.email"
          :subtitle="subtitle"
          :parent="{ label: 'Users', to: { name: 'users' } }">
          <template #meta>
            <DbBadge v-if="user.status === 'disabled'" tone="crit" class="mt-2">
              Disabled
            </DbBadge>
          </template>
          <template v-if="canManage" #actions>
            <DbButton data-testid="edit-user" @click="editOpen = true">
              <PencilIcon class="h-4 w-4" />
              Edit
            </DbButton>
          </template>
        </PageHeader>

        <DbTabNav :tabs="tabs" :active="activeTab" label="Account sections" />

        <!-- Overview -->
        <div
          v-if="activeTab === 'overview'"
          class="grid gap-4 lg:grid-cols-2"
          data-testid="user-overview">
          <section :class="card" aria-labelledby="account-heading">
            <h2
              id="account-heading"
              class="mb-3 text-[15px] font-semibold text-ink-strong">
              Account
            </h2>
            <dl
              class="grid grid-cols-[minmax(0,9rem)_1fr] gap-x-4 gap-y-2.5 text-sm">
              <dt class="text-ink-muted">Email</dt>
              <dd class="font-mono text-ink-strong [overflow-wrap:anywhere]">
                {{ user.email }}
              </dd>
              <dt class="text-ink-muted">Role</dt>
              <dd class="flex flex-wrap items-center gap-1.5">
                <DbBadge :tone="isRoot ? 'accent' : 'neutral'">
                  {{ user.roleName ?? user.role }}
                </DbBadge>
                <span
                  v-if="role && !role.builtin"
                  class="font-mono text-xs text-ink-faint">
                  custom
                </span>
              </dd>
              <dt class="text-ink-muted">Reaches at most</dt>
              <dd>
                <DbLevelBadge
                  v-if="access"
                  v-bind="levelMeter(access.maxLevel)" />
              </dd>
              <dt class="text-ink-muted">Signs in with</dt>
              <dd class="font-mono text-ink">
                {{ SIGN_IN_LABELS[signInMethod(user)] }}
                <span
                  v-if="user.ssoLinked && user.hasPassword"
                  class="text-ink-muted">
                  + sso
                </span>
              </dd>
              <dt class="text-ink-muted">Created</dt>
              <dd class="text-ink">{{ created }}</dd>
            </dl>
          </section>
          <section :class="card" aria-labelledby="reach-heading">
            <h2
              id="reach-heading"
              class="mb-3 text-[15px] font-semibold text-ink-strong">
              What they reach
            </h2>
            <p class="text-sm text-ink">
              {{ access?.grants.length ?? 0 }}
              {{ access?.grants.length === 1 ? "grant" : "grants" }}, in
              {{ access?.groups.length ?? 0 }}
              {{ access?.groups.length === 1 ? "group" : "groups" }}. Their role
              allows up to {{ formatLevel(access?.maxLevel ?? "none") }}.
            </p>
            <RouterLink
              :to="tabs.find((tab) => tab.key === 'access')!.to"
              class="mt-3 inline-block text-sm font-medium text-accent-strong hover:underline">
              See their access ›
            </RouterLink>
          </section>
        </div>

        <!-- Access -->
        <AccessGrantsPanel
          v-else-if="activeTab === 'access' && access"
          :principal="principal"
          :grants="access.grants"
          :summary="accessSummary"
          :read-only="isRoot"
          @changed="controller.reloadAccess" />

        <!-- Groups -->
        <div
          v-else-if="activeTab === 'groups'"
          class="flex flex-col gap-4"
          data-testid="user-groups">
          <div v-if="canManageGroups" class="flex">
            <DbMenu v-if="joinable.length" align="start">
              <template #trigger="{ toggle, attrs }">
                <DbButton v-bind="attrs" @click="toggle">
                  <PlusIcon class="h-4 w-4" />
                  Add to a group
                </DbButton>
              </template>
              <DbMenuItem
                v-for="group in joinable"
                :key="group.id"
                @select="controller.joinGroup(group.id)">
                <template #icon><UsersIcon class="h-4 w-4" /></template>
                {{ group.name }}
              </DbMenuItem>
            </DbMenu>
          </div>
          <DbAlert v-if="groupsError">{{ groupsError }}</DbAlert>
          <div
            v-if="!access?.groups.length"
            class="rounded-card border border-dashed border-line px-6 py-10 text-center text-sm text-ink-muted">
            {{ user.email }} isn't in any group.
          </div>
          <ul
            v-else
            class="divide-y divide-line-soft rounded-card border border-line bg-panel">
            <li
              v-for="group in access.groups"
              :key="group.id"
              class="flex h-[52px] items-center gap-3 px-4">
              <UsersIcon class="h-4 w-4 text-ink-muted" />
              <RouterLink
                :to="{ name: 'group', params: { groupId: group.id } }"
                class="flex-1 font-medium text-ink-strong hover:underline">
                {{ group.name }}
              </RouterLink>
              <DbButton
                v-if="canManageGroups"
                variant="ghost"
                size="sm"
                @click="controller.leaveGroup(group.id)">
                <XIcon class="h-4 w-4" />
                Take out
              </DbButton>
            </li>
          </ul>
        </div>

        <!-- Tokens -->
        <div
          v-else-if="activeTab === 'tokens'"
          class="flex flex-col gap-4"
          data-testid="user-tokens">
          <p class="text-sm text-ink-muted">
            Personal tokens {{ user.email }} made for scripts and agents on
            their own machine. They never reach more than {{ user.email }}
            does.
          </p>
          <DbAlert v-if="tokensError">{{ tokensError }}</DbAlert>
          <DbSkeleton v-if="tokens === null" class="h-32 w-full rounded-card" />
          <div
            v-else-if="tokenRows.length === 0"
            class="rounded-card border border-dashed border-line px-6 py-10 text-center text-sm text-ink-muted">
            No personal tokens.
          </div>
          <div
            v-else
            class="overflow-x-auto rounded-card border border-line bg-panel">
            <table class="min-w-full text-left text-sm">
              <thead>
                <tr class="border-b border-line-soft bg-raised/50">
                  <th class="h-11 px-4 font-medium text-ink-muted">Token</th>
                  <th class="h-11 px-4 font-medium text-ink-muted">Scope</th>
                  <th class="h-11 px-4 font-medium text-ink-muted">Can</th>
                  <th class="h-11 px-4 font-medium text-ink-muted">Expires</th>
                  <th class="h-11 px-4 font-medium text-ink-muted">
                    Last used
                  </th>
                  <th class="h-11 w-24 px-2">
                    <span class="sr-only">Actions</span>
                  </th>
                </tr>
              </thead>
              <tbody>
                <tr
                  v-for="row in tokenRows"
                  :key="row.token.id"
                  class="border-b border-line-soft last:border-b-0">
                  <td class="h-[52px] px-4">
                    <span class="inline-flex items-center gap-2">
                      <span class="font-mono text-ink-strong">
                        {{ row.token.name }}
                      </span>
                      <span
                        class="rounded-control px-1.5 py-0.5 font-mono text-xs"
                        :class="stateClass[row.state]">
                        {{ row.state }}
                      </span>
                    </span>
                  </td>
                  <td class="px-4 font-mono text-ink">{{ row.scope }}</td>
                  <td class="px-4">
                    <DbLevelBadge v-bind="levelMeter(row.token.level)" />
                  </td>
                  <td class="px-4 text-ink-muted">{{ row.expires }}</td>
                  <td class="px-4 text-ink-muted">{{ row.lastUsed }}</td>
                  <td class="px-2 text-right">
                    <DbButton
                      v-if="canRevokeTokens && row.state === 'active'"
                      variant="ghost"
                      size="sm"
                      @click="controller.revokeToken(row.token)">
                      Revoke
                    </DbButton>
                  </td>
                </tr>
              </tbody>
            </table>
          </div>
        </div>

        <!-- Danger zone -->
        <div
          v-else-if="activeTab === 'danger' && canManage"
          class="flex flex-col gap-3 rounded-card border border-crit/60 p-4 sm:p-5"
          data-testid="user-danger">
          <div
            v-if="user.ssoLinked"
            class="flex flex-wrap items-center justify-between gap-3 border-b border-line-soft pb-3">
            <div>
              <p class="text-sm font-medium text-ink-strong">
                Unlink single sign-on
              </p>
              <p class="text-sm text-ink-muted">
                Their next single sign-on links again if the provider vouches
                for their email.
              </p>
            </div>
            <DbButton @click="actions.ask('unlink', user)">Unlink</DbButton>
          </div>
          <div
            class="flex flex-wrap items-center justify-between gap-3 border-b border-line-soft pb-3">
            <div>
              <p class="text-sm font-medium text-ink-strong">
                {{
                  user.status === "disabled"
                    ? "Turn the account back on"
                    : "Disable the account"
                }}
              </p>
              <p class="text-sm text-ink-muted">
                {{
                  user.status === "disabled"
                    ? "They can sign in again with the access they had."
                    : "Signs them out everywhere and stops their tokens. Nothing is deleted."
                }}
              </p>
            </div>
            <DbButton
              v-if="user.status === 'disabled'"
              data-testid="enable-user"
              @click="actions.ask('enable', user)">
              Turn back on
            </DbButton>
            <DbButton
              v-else
              variant="danger"
              data-testid="disable-user"
              @click="actions.ask('disable', user)">
              Disable account
            </DbButton>
          </div>
          <div class="flex flex-wrap items-center justify-between gap-3">
            <div>
              <p class="text-sm font-medium text-ink-strong">
                Delete the account
              </p>
              <p class="text-sm text-ink-muted">
                Their grants, groups and tokens go with it. Audit events stay.
              </p>
            </div>
            <DbButton
              variant="danger"
              data-testid="delete-user"
              @click="actions.ask('delete', user)">
              Delete user
            </DbButton>
          </div>
        </div>
      </template>
    </div>

    <UserDialog
      v-if="user"
      :open="editOpen"
      :roles="roles"
      :sso-enabled="ssoEnabled"
      :editing="user"
      @close="editOpen = false"
      @saved="controller.edited" />

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
