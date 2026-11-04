<script setup lang="ts">
import { usePersonalTokensCardController } from "./PersonalTokensCard.controller";
import PersonalTokenDialog from "./PersonalTokenDialog.vue";
import OneTimeTokenDialog from "~/components/app/OneTimeTokenDialog.vue";
import {
  DbAlert,
  DbBadge,
  DbButton,
  DbConfirmDialog,
  DbCopyButton,
  DbLevelBadge,
  DbMenu,
  DbMenuItem,
} from "~/components/ui";
import { PlusIcon, TerminalIcon } from "~/assets/icons";
import { formatDate } from "~/utils/format";

/**
 * The Account page's tokens tab: Personal tokens, or Audit tokens for
 * roles whose tokens read the audit log. A table of live tokens, New
 * token, and the token shown once after it's made.
 */
const controller = usePersonalTokensCardController();
const {
  rows,
  projects,
  environments,
  limits,
  error,
  notice,
  created,
  dialogOpen,
  visible,
  auditOnly,
  revoking,
  revokeBusy,
  revokeError,
  activity,
  pullCommand,
} = controller;
</script>

<template>
  <section
    v-if="visible"
    class="flex flex-col gap-4"
    :aria-label="auditOnly ? 'Audit tokens' : 'Personal tokens'"
    data-testid="personal-tokens-card">
    <p class="text-sm text-ink-muted">
      <template v-if="auditOnly">
        Your role's tokens read the audit log only. Use one for a SIEM or log
        collector. They can't see projects or secrets.
      </template>
      <template v-else>
        A personal token acts as you from a script or an AI agent. It never
        reaches more than you, and losing access to a project takes it from your
        tokens too.
      </template>
    </p>
    <div class="flex justify-end">
      <DbButton
        variant="primary"
        data-testid="new-personal-token"
        @click="dialogOpen = true">
        <PlusIcon class="h-4 w-4" />
        {{ auditOnly ? "New audit token" : "New token" }}
      </DbButton>
    </div>

    <DbAlert v-if="error">{{ error }}</DbAlert>
    <DbAlert v-if="notice" tone="success">{{ notice }}</DbAlert>

    <div
      v-if="rows.length === 0"
      class="rounded-card border border-dashed border-line px-6 py-10 text-center text-sm text-ink-muted">
      No live tokens.
      {{
        auditOnly
          ? "Create one for whatever pulls the audit log."
          : "Create one for a script or an agent on your machine."
      }}
    </div>
    <div
      v-else
      class="overflow-hidden rounded-card border border-line bg-panel">
      <table
        class="w-full table-fixed text-left text-sm"
        data-testid="personal-tokens">
        <thead>
          <tr class="border-b border-line-soft">
            <th class="h-10 px-4 font-medium text-ink-muted">Name</th>
            <th
              class="h-10 w-40 px-4 font-medium text-ink-muted max-sm:hidden"
              :class="auditOnly && 'w-44'">
              {{ auditOnly ? "Reads" : "Can" }}
            </th>
            <th
              v-if="!auditOnly"
              class="hidden h-10 w-40 px-4 font-medium text-ink-muted md:table-cell">
              Where
            </th>
            <th
              class="hidden h-10 w-28 px-4 font-medium text-ink-muted md:table-cell">
              Last used
            </th>
            <th
              class="hidden h-10 w-32 px-4 font-medium text-ink-muted sm:table-cell">
              Expires
            </th>
            <th class="h-10 w-12 px-2"><span class="sr-only">Actions</span></th>
          </tr>
        </thead>
        <tbody>
          <tr
            v-for="row in rows"
            :key="row.token.id"
            class="border-b border-line-soft last:border-b-0"
            data-testid="personal-token-row">
            <td class="h-[52px] px-4 py-2">
              <p class="truncate font-mono text-ink-strong">
                {{ row.token.name }}
              </p>
              <p class="truncate text-xs text-ink-muted sm:hidden">
                <template v-if="!auditOnly">{{ row.can.label }} · </template>
                expires {{ row.expires }}
              </p>
            </td>
            <td class="px-4 max-sm:hidden">
              <DbBadge v-if="auditOnly" tone="accent">audit log</DbBadge>
              <DbLevelBadge v-else v-bind="row.can" />
            </td>
            <td
              v-if="!auditOnly"
              class="hidden truncate px-4 font-mono text-ink md:table-cell">
              {{ row.where }}
            </td>
            <td class="hidden px-4 text-ink md:table-cell">
              {{ row.lastUsed }}
            </td>
            <td class="hidden px-4 text-ink sm:table-cell">
              {{ row.expires }}
            </td>
            <td class="px-2 text-right">
              <DbMenu :label="`Actions for ${row.token.name}`">
                <template v-if="auditOnly">
                  <DbMenuItem @select="controller.copyName(row.token)">
                    Copy name
                  </DbMenuItem>
                  <DbMenuItem @select="$router.push(activity)">
                    Activity
                  </DbMenuItem>
                  <div class="my-0.5 h-px bg-line-soft" role="separator" />
                </template>
                <DbMenuItem
                  tone="danger"
                  @select="controller.askRevoke(row.token)">
                  Revoke token
                </DbMenuItem>
              </DbMenu>
            </td>
          </tr>
        </tbody>
      </table>
    </div>

    <div
      v-if="auditOnly"
      class="rounded-card border border-line bg-editor"
      data-testid="audit-token-usage">
      <div
        class="flex items-center gap-2 border-b border-line-soft py-1 pl-3 pr-1">
        <TerminalIcon class="h-3.5 w-3.5 text-ink-muted" />
        <p class="flex-1 text-sm font-medium text-ink-muted">Pull events</p>
        <DbCopyButton :value="pullCommand" label="Copy command" icon-only />
      </div>
      <pre
        class="overflow-x-auto p-3 font-mono text-xs leading-relaxed text-ink-strong"
        >{{ pullCommand }}</pre>
    </div>

    <PersonalTokenDialog
      :open="dialogOpen"
      :audit-only="auditOnly"
      :projects="projects"
      :environments="environments"
      :limits="limits"
      @close="dialogOpen = false"
      @created="controller.tokenCreated" />

    <OneTimeTokenDialog
      :id="created?.token.id ?? ''"
      :open="created !== null"
      title="Copy your new token"
      :token="created?.plaintextToken ?? ''"
      :name="created?.token.name ?? ''"
      :scope="
        created?.token.scopeType === 'audit'
          ? 'The audit log you see'
          : created?.token.scopeLabel || 'Every project you reach'
      "
      :expires="
        created?.token.expiresAt ? formatDate(created.token.expiresAt) : 'Never'
      "
      :usage="`export DOPBASE_TOKEN=${created?.plaintextToken ?? ''}`"
      @acknowledge="controller.acknowledge" />

    <DbConfirmDialog
      :open="revoking !== null"
      title="Revoke this token?"
      :description="`${revoking?.name ?? ''} stops working right away. Anything using it will need a new one.`"
      confirm-label="Revoke token"
      :loading="revokeBusy"
      :error="revokeError"
      @confirm="controller.confirmRevoke"
      @close="controller.cancelRevoke" />
  </section>
</template>
