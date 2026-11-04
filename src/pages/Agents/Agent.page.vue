<script setup lang="ts">
import { computed } from "vue";
import { HISTORY_SHOWN, useAgentController } from "./Agent.controller";
import { agentTokenUsage } from "./Agents.controller";
import AgentTokenDialog from "./components/AgentTokenDialog.vue";
import TransferOwnerDialog from "./components/TransferOwnerDialog.vue";
import { DashboardLayout } from "~/layouts";
import PageHeader from "~/components/app/PageHeader.vue";
import AccessGrantsPanel from "~/components/app/AccessGrantsPanel.vue";
import OneTimeTokenDialog from "~/components/app/OneTimeTokenDialog.vue";
import {
  DbAlert,
  DbBadge,
  DbButton,
  DbConfirmDialog,
  DbLevelBadge,
  DbSkeleton,
  DbTabNav,
} from "~/components/ui";
import { ActivityIcon, KeyIcon, UserIcon } from "~/assets/icons";
import type { TokenState } from "~/pages/Users/people";
import { formatDate } from "~/utils/format";
import { levelMeter } from "~/utils/level";

const controller = useAgentController();
const {
  activeTab,
  tabs,
  agent,
  access,
  role,
  roles,
  people,
  loadError,
  canManage,
  canTransfer,
  subtitle,
  principal,
  accessSummary,
  current,
  tokenRows,
  liveRows,
  historyRows,
  shownHistory,
  showAllHistory,
  olderCount,
  tokensError,
  tokenOpen,
  createdToken,
  transferOpen,
  actions,
  activity,
} = controller;
const { pending, copy, busy, error: actionError } = actions;

const stateTone: Record<TokenState, "ok" | "neutral" | "crit"> = {
  active: "ok",
  expired: "neutral",
  revoked: "crit",
};
/**
 * Columns that drop on a narrow table, by the table's own width. Hidden
 * cells stay in the grid (display:none would shift the rest left under the
 * wrong column) with zero width and visibility:hidden.
 */
const narrowHidden = {
  can: "invisible overflow-hidden p-0 @lg:visible @lg:px-4",
  expires: "invisible overflow-hidden p-0 @2xl:visible @2xl:px-4",
  where: "invisible overflow-hidden p-0 @3xl:visible @3xl:px-4",
  lastUsed: "invisible overflow-hidden p-0 @4xl:visible @4xl:px-4",
  created: "invisible overflow-hidden p-0 @5xl:visible @5xl:px-4",
};
const usage = computed(() =>
  createdToken.value ? agentTokenUsage(createdToken.value) : "",
);
</script>

<template>
  <DashboardLayout>
    <div class="page-width flex flex-col gap-5 py-6 lg:py-8">
      <div v-if="loadError" class="flex flex-col items-start gap-3">
        <DbAlert>{{ loadError }}</DbAlert>
        <DbButton size="sm" @click="controller.load">Try again</DbButton>
      </div>

      <div v-else-if="!agent" class="flex flex-col gap-3">
        <DbSkeleton class="h-8 w-72 rounded-control" />
        <DbSkeleton class="h-48 w-full rounded-card" />
      </div>

      <template v-else>
        <PageHeader
          :title="agent.email"
          :subtitle="subtitle"
          :parent="{ label: 'AI agents', to: { name: 'agents' } }">
          <template #actions>
            <DbButton variant="ghost" @click="$router.push(activity)">
              <ActivityIcon class="h-4 w-4" />
              Activity
            </DbButton>
            <DbButton
              v-if="canTransfer"
              data-testid="transfer-owner"
              @click="transferOpen = true">
              <UserIcon class="h-4 w-4" />
              Transfer owner
            </DbButton>
          </template>
        </PageHeader>

        <DbTabNav :tabs="tabs" :active="activeTab" label="Agent sections" />

        <AccessGrantsPanel
          v-if="activeTab === 'access' && access"
          :principal="principal"
          :grants="access.grants"
          :give-to="{
            name: 'agent-access-give',
            params: { agentId: principal.id },
          }"
          :summary="accessSummary"
          @changed="controller.reloadAccess" />

        <div
          v-else-if="activeTab === 'tokens'"
          class="flex flex-col gap-4"
          data-testid="agent-tokens">
          <div
            class="flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between">
            <p class="max-w-2xl text-sm text-ink-muted">
              An agent holds one token at a time. A new one revokes the old, so
              a copy left in a script or leaked somewhere stops working. Past
              tokens stay below as history.
            </p>
            <DbButton
              v-if="canManage"
              variant="primary"
              class="shrink-0 self-start"
              data-testid="new-agent-token"
              @click="tokenOpen = true">
              <KeyIcon class="h-4 w-4" />
              New token
            </DbButton>
          </div>
          <DbAlert v-if="tokensError">{{ tokensError }}</DbAlert>
          <div
            v-if="!tokenRows.length"
            class="rounded-card border border-dashed border-line px-6 py-10 text-center text-sm text-ink-muted">
            No token yet. Create one when the agent has the access it needs.
          </div>
          <!-- Columns drop by the table's own width, so Token always keeps room. -->
          <div
            v-else
            class="@container overflow-hidden rounded-card border border-line bg-panel">
            <table class="w-full table-fixed text-left text-sm">
              <!-- A dropped column keeps width 0, so Token gets the room. -->
              <colgroup>
                <col />
                <col class="w-24" />
                <col class="w-0 @lg:w-28" />
                <col class="w-0 @3xl:w-60" />
                <col class="w-0 @5xl:w-32" />
                <col class="w-0 @2xl:w-32" />
                <col class="w-0 @4xl:w-28" />
                <col class="w-24" />
              </colgroup>
              <thead>
                <tr
                  class="whitespace-nowrap border-b border-line-soft bg-raised/50">
                  <th class="h-11 px-4 font-medium text-ink-muted">Token</th>
                  <th class="h-11 px-3 font-medium text-ink-muted">Status</th>
                  <th
                    class="h-11 font-medium text-ink-muted"
                    :class="narrowHidden.can">
                    Can
                  </th>
                  <th
                    class="h-11 font-medium text-ink-muted"
                    :class="narrowHidden.where">
                    Where
                  </th>
                  <th
                    class="h-11 font-medium text-ink-muted"
                    :class="narrowHidden.created">
                    Created
                  </th>
                  <th
                    class="h-11 font-medium text-ink-muted"
                    :class="narrowHidden.expires">
                    Expires
                  </th>
                  <th
                    class="h-11 font-medium text-ink-muted"
                    :class="narrowHidden.lastUsed">
                    Last used
                  </th>
                  <th class="h-11 px-2">
                    <span class="sr-only">Actions</span>
                  </th>
                </tr>
              </thead>
              <tbody>
                <tr v-if="!liveRows.length" class="border-b border-line-soft">
                  <td colspan="8" class="h-[52px] px-4 text-ink-muted">
                    No live token. The agent can't sign in until it has one.
                  </td>
                </tr>
                <template
                  v-for="row in [...liveRows, ...shownHistory]"
                  :key="row.token.id">
                  <tr
                    class="border-b border-line-soft last:border-b-0"
                    data-testid="agent-token-row">
                    <td class="h-[52px] px-4 py-2">
                      <p
                        class="truncate font-mono"
                        :class="
                          row.state === 'active'
                            ? 'text-ink-strong'
                            : 'text-ink-faint'
                        "
                        :title="row.token.name">
                        {{ row.token.name }}
                      </p>
                      <p
                        class="truncate font-mono text-xs @3xl:hidden"
                        :class="
                          row.state === 'active'
                            ? 'text-ink-muted'
                            : 'text-ink-faint'
                        "
                        :title="row.scope">
                        {{ row.scope }}
                      </p>
                    </td>
                    <td class="px-3">
                      <DbBadge :tone="stateTone[row.state]">
                        {{ row.state }}
                      </DbBadge>
                    </td>
                    <td :class="narrowHidden.can">
                      <DbLevelBadge v-bind="levelMeter(row.token.level)" />
                    </td>
                    <td :class="narrowHidden.where">
                      <p
                        class="truncate font-mono"
                        :class="
                          row.state === 'active' ? 'text-ink' : 'text-ink-faint'
                        "
                        :title="row.scope">
                        {{ row.scope }}
                      </p>
                    </td>
                    <td
                      class="whitespace-nowrap text-ink-muted"
                      :class="narrowHidden.created">
                      {{ row.created }}
                    </td>
                    <td
                      class="whitespace-nowrap text-ink-muted"
                      :class="narrowHidden.expires">
                      {{ row.expires }}
                    </td>
                    <td
                      class="truncate text-ink-muted"
                      :class="narrowHidden.lastUsed">
                      {{ row.lastUsed }}
                    </td>
                    <td class="px-2 text-right">
                      <DbButton
                        v-if="canManage && row.state === 'active'"
                        variant="ghost"
                        size="sm"
                        @click="controller.revokeToken(row.token)">
                        Revoke
                      </DbButton>
                    </td>
                  </tr>
                </template>
                <tr
                  v-if="historyRows.length > HISTORY_SHOWN"
                  class="border-t border-line-soft">
                  <td colspan="8" class="px-2 py-1.5">
                    <DbButton
                      variant="ghost"
                      size="sm"
                      data-testid="agent-token-older"
                      @click="showAllHistory = !showAllHistory">
                      {{
                        showAllHistory
                          ? "Show fewer"
                          : `Show ${olderCount} older`
                      }}
                    </DbButton>
                  </td>
                </tr>
              </tbody>
            </table>
          </div>
        </div>

        <div
          v-else-if="activeTab === 'danger' && canManage"
          class="flex flex-col gap-3 rounded-card border border-crit/60 p-4 sm:p-5"
          data-testid="agent-danger">
          <div
            class="flex flex-wrap items-center justify-between gap-3 border-b border-line-soft pb-3">
            <div>
              <p class="text-sm font-medium text-ink-strong">
                {{
                  agent.status === "disabled"
                    ? "Turn the agent back on"
                    : "Disable the agent"
                }}
              </p>
              <p class="text-sm text-ink-muted">
                {{
                  agent.status === "disabled"
                    ? "Its token works again if it hasn't expired."
                    : "Its token stops working. Its access and history stay."
                }}
              </p>
            </div>
            <DbButton
              v-if="agent.status === 'disabled'"
              @click="actions.ask('enable', agent)">
              Turn back on
            </DbButton>
            <DbButton
              v-else
              variant="danger"
              @click="actions.ask('disable', agent)">
              Disable agent
            </DbButton>
          </div>
          <div class="flex flex-wrap items-center justify-between gap-3">
            <div>
              <p class="text-sm font-medium text-ink-strong">
                Delete the agent
              </p>
              <p class="text-sm text-ink-muted">
                Its grants and tokens go with it. Audit events stay.
              </p>
            </div>
            <DbButton
              variant="danger"
              data-testid="delete-agent"
              @click="actions.ask('delete', agent)">
              Delete agent
            </DbButton>
          </div>
        </div>
      </template>
    </div>

    <AgentTokenDialog
      v-if="agent"
      :open="tokenOpen"
      :agent="agent"
      :role="role"
      :grants="access?.grants ?? []"
      :current="current"
      @close="tokenOpen = false"
      @created="controller.tokenCreated" />

    <OneTimeTokenDialog
      :id="createdToken?.token.id ?? ''"
      :open="createdToken !== null"
      title="Copy the agent's token"
      :name="createdToken?.token.name ?? ''"
      :token="createdToken?.plaintextToken ?? ''"
      :scope="createdToken?.token.scopeLabel || 'Every project it reaches'"
      :expires="
        createdToken?.token.expiresAt
          ? formatDate(createdToken.token.expiresAt)
          : 'Never'
      "
      :usage="usage"
      @acknowledge="createdToken = null" />

    <TransferOwnerDialog
      :open="transferOpen"
      :agent="agent"
      :people="people"
      :roles="roles"
      @close="transferOpen = false"
      @saved="controller.transferred" />

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
