<script setup lang="ts">
import { computed } from "vue";
import { useAgentController } from "./Agent.controller";
import { agentTokenUsage } from "./Agents.controller";
import AgentTokenDialog from "./components/AgentTokenDialog.vue";
import TransferOwnerDialog from "./components/TransferOwnerDialog.vue";
import { DashboardLayout } from "~/layouts";
import PageHeader from "~/components/app/PageHeader.vue";
import AccessGrantsPanel from "~/components/app/AccessGrantsPanel.vue";
import OneTimeTokenDialog from "~/components/app/OneTimeTokenDialog.vue";
import {
  DbAlert,
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
  people,
  loadError,
  canManage,
  subtitle,
  principal,
  accessSummary,
  current,
  tokenRows,
  tokensError,
  tokenOpen,
  createdToken,
  transferOpen,
  actions,
  activity,
} = controller;
const { pending, copy, busy, error: actionError } = actions;

const stateClass: Record<TokenState, string> = {
  active: "bg-ok/15 text-ok",
  expired: "bg-raised text-ink-muted",
  revoked: "bg-crit/15 text-crit",
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
              v-if="canManage"
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
          :summary="accessSummary"
          @changed="controller.reloadAccess" />

        <div
          v-else-if="activeTab === 'tokens'"
          class="flex flex-col gap-4"
          data-testid="agent-tokens">
          <div class="flex flex-wrap items-center justify-between gap-3">
            <p class="text-sm text-ink-muted">
              An agent holds one token at a time. A new one replaces it.
            </p>
            <DbButton
              v-if="canManage"
              variant="primary"
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
          <div
            v-else
            class="overflow-x-auto rounded-card border border-line bg-panel">
            <table class="min-w-full text-left text-sm">
              <thead>
                <tr class="border-b border-line-soft bg-raised/50">
                  <th class="h-11 px-4 font-medium text-ink-muted">Token</th>
                  <th class="h-11 px-4 font-medium text-ink-muted">Can</th>
                  <th class="h-11 px-4 font-medium text-ink-muted">Where</th>
                  <th class="h-11 px-4 font-medium text-ink-muted">Created</th>
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
                  class="border-b border-line-soft last:border-b-0"
                  data-testid="agent-token-row">
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
                  <td class="px-4">
                    <DbLevelBadge v-bind="levelMeter(row.token.level)" />
                  </td>
                  <td class="px-4 font-mono text-ink">{{ row.scope }}</td>
                  <td class="px-4 text-ink-muted">{{ row.created }}</td>
                  <td class="px-4 text-ink-muted">{{ row.expires }}</td>
                  <td class="px-4 text-ink-muted">{{ row.lastUsed }}</td>
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
