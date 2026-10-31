<script setup lang="ts">
import { computed } from "vue";
import {
  agentTokenUsage,
  useAgentsController,
  type AgentTokenStatus,
} from "./Agents.controller";
import AddAgentDialog from "./components/AddAgentDialog.vue";
import AgentTokenDialog from "./components/AgentTokenDialog.vue";
import TransferOwnerDialog from "./components/TransferOwnerDialog.vue";
import { DashboardLayout } from "~/layouts";
import PageHeader from "~/components/app/PageHeader.vue";
import OneTimeTokenDialog from "~/components/app/OneTimeTokenDialog.vue";
import {
  DbAlert,
  DbButton,
  DbConfirmDialog,
  DbFilterMenu,
  DbLevelBadge,
  DbMenu,
  DbMenuItem,
  DbSearchField,
  DbSkeleton,
} from "~/components/ui";
import {
  ActivityIcon,
  BotIcon,
  KeyIcon,
  PlusIcon,
  TrashIcon,
  UserIcon,
  XIcon,
} from "~/assets/icons";
import { formatDate } from "~/utils/format";

const controller = useAgentsController();
const {
  agents,
  people,
  roles,
  loadError,
  filter,
  ownerFilter,
  ownerFilters,
  visibleRows,
  agentRole,
  canAdd,
  addOpen,
  tokenFor,
  tokenGrants,
  createdToken,
  revoking,
  revokeBusy,
  revokeError,
  transferring,
  actions,
} = controller;
const { pending, copy, busy, error: actionError } = actions;

const statusClass: Record<AgentTokenStatus, string> = {
  active: "bg-ok/15 text-ok",
  expiring: "bg-warn/15 text-warn",
  none: "bg-raised text-ink-muted",
  unknown: "bg-raised text-ink-muted",
};

const tokenRole = computed(
  () =>
    roles.value.find((role) => role.id === tokenFor.value?.agent.roleId) ??
    agentRole.value,
);
const tokenUsage = computed(() =>
  createdToken.value ? agentTokenUsage(createdToken.value) : "",
);
</script>

<template>
  <DashboardLayout>
    <div class="page-width flex flex-col gap-5 py-6 lg:py-8">
      <PageHeader
        title="AI agents"
        subtitle="Token-only accounts for bots and CI. Each has an owner and never reaches more than that person.">
        <template v-if="canAdd" #actions>
          <DbButton
            variant="primary"
            data-testid="add-agent"
            @click="addOpen = true">
            <PlusIcon class="h-4 w-4" />
            Add AI agent
          </DbButton>
        </template>
      </PageHeader>

      <div class="flex flex-wrap items-center gap-2">
        <DbSearchField
          v-model="filter"
          class="w-full sm:w-64"
          placeholder="Filter agents" />
        <DbFilterMenu
          v-model="ownerFilter"
          label="Owner"
          :options="ownerFilters" />
      </div>

      <div v-if="loadError" class="flex flex-col items-start gap-3">
        <DbAlert>{{ loadError }}</DbAlert>
        <DbButton size="sm" @click="controller.load">Try again</DbButton>
      </div>

      <div v-else-if="agents === null" class="flex flex-col gap-2">
        <DbSkeleton
          v-for="line in 3"
          :key="line"
          class="h-12 w-full rounded-control" />
      </div>

      <template v-else>
        <div
          v-if="agents.length === 0"
          class="rounded-card border border-dashed border-line px-6 py-10 text-center">
          <p class="text-sm font-medium text-ink-strong">No AI agents yet</p>
          <p class="mt-1 text-sm text-ink-muted">
            Add one for a bot or CI job that needs its own token and access.
          </p>
        </div>

        <!-- Table on wider screens -->
        <div
          v-else
          class="hidden overflow-x-auto rounded-card border border-line bg-panel sm:block">
          <table
            class="min-w-full text-left text-sm"
            data-testid="agents-table">
            <thead>
              <tr class="border-b border-line-soft bg-raised/50">
                <th class="h-11 px-4 font-medium text-ink-muted">Agent</th>
                <th class="h-11 px-4 font-medium text-ink-muted">Owner</th>
                <th class="h-11 px-4 font-medium text-ink-muted">
                  Reaches at most
                </th>
                <th class="h-11 px-4 font-medium text-ink-muted">Token</th>
                <th class="h-11 px-4 font-medium text-ink-muted">Expires</th>
                <th class="h-11 px-4 font-medium text-ink-muted">Last used</th>
                <th class="h-11 w-12 px-2">
                  <span class="sr-only">Actions</span>
                </th>
              </tr>
            </thead>
            <tbody>
              <tr
                v-for="row in visibleRows"
                :key="row.agent.id"
                class="border-b border-line-soft last:border-b-0"
                data-testid="agent-row">
                <td class="h-[52px] px-4">
                  <RouterLink
                    :to="{ name: 'agent', params: { agentId: row.agent.id } }"
                    class="inline-flex items-center gap-2 font-mono text-ink-strong hover:underline">
                    <BotIcon class="h-4 w-4 text-ink-muted" />
                    {{ row.agent.email }}
                  </RouterLink>
                </td>
                <td class="px-4 font-mono text-ink">{{ row.owner }}</td>
                <td class="px-4">
                  <DbLevelBadge v-if="row.reach" v-bind="row.reach" />
                </td>
                <td class="px-4">
                  <span
                    class="rounded-control px-1.5 py-0.5 font-mono text-xs"
                    :class="statusClass[row.status]">
                    {{ row.statusLabel }}
                  </span>
                </td>
                <td class="px-4 text-ink-muted">{{ row.expires }}</td>
                <td class="px-4 text-ink-muted">{{ row.lastUsed }}</td>
                <td class="px-2 text-right">
                  <DbMenu
                    v-if="row.canManage"
                    :label="`Actions for ${row.agent.email}`">
                    <DbMenuItem @select="controller.openToken(row)">
                      <template #icon><KeyIcon class="h-4 w-4" /></template>
                      New token
                    </DbMenuItem>
                    <DbMenuItem @select="transferring = row.agent">
                      <template #icon><UserIcon class="h-4 w-4" /></template>
                      Transfer owner
                    </DbMenuItem>
                    <DbMenuItem @select="$router.push(row.activity)">
                      <template #icon
                        ><ActivityIcon class="h-4 w-4"
                      /></template>
                      Activity
                    </DbMenuItem>
                    <div class="my-0.5 h-px bg-line-soft" role="separator" />
                    <DbMenuItem
                      v-if="row.token"
                      tone="danger"
                      @select="controller.askRevoke(row)">
                      <template #icon><XIcon class="h-4 w-4" /></template>
                      Revoke token
                    </DbMenuItem>
                    <DbMenuItem
                      tone="danger"
                      @select="actions.ask('delete', row.agent)">
                      <template #icon><TrashIcon class="h-4 w-4" /></template>
                      Delete agent
                    </DbMenuItem>
                  </DbMenu>
                </td>
              </tr>
            </tbody>
          </table>
          <p
            v-if="visibleRows.length === 0"
            class="px-4 py-6 text-center text-sm text-ink-muted">
            No agents match these filters.
          </p>
        </div>

        <!-- Cards on phones -->
        <ul
          v-if="agents.length > 0"
          class="flex flex-col gap-2 sm:hidden"
          data-testid="agents-cards">
          <li
            v-for="row in visibleRows"
            :key="row.agent.id"
            class="rounded-card border border-line bg-panel p-3">
            <RouterLink
              :to="{ name: 'agent', params: { agentId: row.agent.id } }"
              class="flex flex-col gap-1.5">
              <span
                class="inline-flex items-center gap-2 font-mono text-sm text-ink-strong">
                <BotIcon class="h-4 w-4 text-ink-muted" />
                {{ row.agent.email }}
              </span>
              <span class="text-xs text-ink-muted">Owner {{ row.owner }}</span>
              <span class="flex flex-wrap items-center gap-2 text-xs">
                <span
                  class="rounded-control px-1.5 py-0.5 font-mono"
                  :class="statusClass[row.status]">
                  {{ row.statusLabel }}
                </span>
                <span class="text-ink-muted">Last used {{ row.lastUsed }}</span>
              </span>
            </RouterLink>
          </li>
        </ul>

        <p
          class="flex items-start gap-2 rounded-card border border-line bg-panel px-4 py-3 text-sm text-ink-muted">
          <KeyIcon class="mt-0.5 h-4 w-4 shrink-0" />
          An agent on your own machine doesn't need an account. Create a
          personal token with Names only from your account menu.
        </p>
      </template>
    </div>

    <AddAgentDialog
      :open="addOpen"
      :role="agentRole"
      @close="addOpen = false"
      @added="controller.added" />

    <AgentTokenDialog
      v-if="tokenFor"
      :open="tokenFor !== null"
      :agent="tokenFor.agent"
      :role="tokenRole"
      :grants="tokenGrants"
      :current="tokenFor.token"
      @close="tokenFor = null"
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
      :usage="tokenUsage"
      @acknowledge="createdToken = null" />

    <TransferOwnerDialog
      :open="transferring !== null"
      :agent="transferring"
      :people="people"
      @close="transferring = null"
      @saved="controller.transferred" />

    <DbConfirmDialog
      :open="revoking !== null"
      :title="`Revoke ${revoking?.token?.name ?? 'the token'}?`"
      :description="`${revoking?.agent.email ?? 'The agent'} stops working wherever this token is used, right away.`"
      confirm-label="Revoke token"
      :loading="revokeBusy"
      :error="revokeError"
      @confirm="controller.confirmRevoke"
      @close="controller.cancelRevoke" />

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
