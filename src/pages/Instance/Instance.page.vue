<script setup lang="ts">
import { useInstanceController } from "./Instance.controller";
import { DashboardLayout } from "~/layouts";
import {
  DbAlert,
  DbBadge,
  DbButton,
  DbSkeleton,
} from "~/components/ui";
import { RefreshIcon, ServerIcon } from "~/assets/icons";

/**
 * Instance — read-only server status: version, database health,
 * master-key availability, uptime, observed date, and resource metrics.
 */
const {
  status,
  loading,
  loadError,
  load,
  healthTone,
  formattedUptime,
  formattedObservedAt,
  relativeObservedAt,
  relativeBootTime,
  resetPreview,
  resetPassword,
  resetConfirmation,
  resetAcknowledged,
  resetLoading,
  resetError,
  resetComplete,
  loadResetPreview,
  performReset,
  isRoot,
} = useInstanceController();
</script>

<template>
  <DashboardLayout>
    <div class="mx-auto max-w-4xl p-8">
      <header class="mb-6 flex items-center justify-between">
        <div>
          <div class="flex items-center gap-2.5">
            <h1 class="text-lg font-semibold text-ink-strong">Instance</h1>
            <DbBadge tone="neutral">Self-hosted</DbBadge>
          </div>
          <p class="mt-1 text-sm text-ink-muted">
            Read-only status and operational telemetry for this server.
          </p>
        </div>
        <DbButton
          size="sm"
          variant="secondary"
          :loading="loading"
          @click="load">
          <RefreshIcon class="h-3.5 w-3.5" />
          Refresh
        </DbButton>
      </header>

      <DbAlert v-if="loadError" class="mb-6">{{ loadError }}</DbAlert>

      <!-- Skeleton loading state -->
      <div v-if="loading && !status" class="space-y-6" data-testid="instance-skeleton">
        <div class="grid grid-cols-1 gap-3.5 sm:grid-cols-2 lg:grid-cols-4">
          <div
            v-for="i in 4"
            :key="i"
            class="rounded-card border border-line bg-panel p-4 space-y-2.5">
            <DbSkeleton class="h-3 w-16" />
            <DbSkeleton class="h-6 w-24" />
            <DbSkeleton class="h-3 w-20" />
          </div>
        </div>

        <DbSkeleton class="h-10 w-full rounded-card" />

        <div class="space-y-3">
          <DbSkeleton class="h-4 w-28" />
          <div class="grid grid-cols-2 gap-3 sm:grid-cols-4">
            <div
              v-for="i in 8"
              :key="i"
              class="rounded-card border border-line bg-panel p-4 space-y-2">
              <DbSkeleton class="h-3 w-20" />
              <DbSkeleton class="h-7 w-12" />
            </div>
          </div>
        </div>
      </div>

      <!-- Loaded status content -->
      <div v-else-if="status" class="space-y-6" data-testid="instance-content">
        <!-- System Health 4-card grid -->
        <div class="grid grid-cols-1 gap-3.5 sm:grid-cols-2 lg:grid-cols-4" data-testid="instance-cards">
          <!-- Version -->
          <div class="flex flex-col justify-between rounded-card border border-line bg-panel p-4">
            <p class="text-[11px] font-semibold tracking-wider text-ink-muted uppercase">
              Core Version
            </p>
            <p class="my-1 font-mono text-lg font-semibold text-ink-strong">
              v{{ status.version }}
            </p>
            <p class="text-xs text-ink-muted">
              {{ status.initializationState === "initialized" ? "Setup complete" : status.initializationState }}
            </p>
          </div>

          <!-- Database -->
          <div class="flex flex-col justify-between rounded-card border border-line bg-panel p-4">
            <p class="text-[11px] font-semibold tracking-wider text-ink-muted uppercase">
              Database
            </p>
            <div class="my-1 flex items-center">
              <DbBadge :tone="healthTone(status.databaseHealth)">
                {{ status.databaseHealth }}
              </DbBadge>
            </div>
            <p class="text-xs text-ink-muted">SQLite WAL storage</p>
          </div>

          <!-- Master key -->
          <div class="flex flex-col justify-between rounded-card border border-line bg-panel p-4">
            <p class="text-[11px] font-semibold tracking-wider text-ink-muted uppercase">
              Master Key
            </p>
            <div class="my-1 flex items-center">
              <DbBadge :tone="healthTone(status.keyAvailability)">
                {{ status.keyAvailability }}
              </DbBadge>
            </div>
            <p class="text-xs text-ink-muted">Envelope encryption</p>
          </div>

          <!-- Uptime -->
          <div class="flex flex-col justify-between rounded-card border border-line bg-panel p-4">
            <p class="text-[11px] font-semibold tracking-wider text-ink-muted uppercase">
              Uptime
            </p>
            <p class="my-1 font-mono text-lg font-semibold text-ink-strong">
              {{ formattedUptime }}
            </p>
            <p class="text-xs text-ink-muted">
              Started {{ relativeBootTime }}
            </p>
          </div>
        </div>

        <!-- Telemetry timestamp banner -->
        <div
          class="flex flex-wrap items-center justify-between gap-2 rounded-card border border-line-soft bg-raised/40 px-4 py-2.5 text-xs text-ink-muted">
          <div class="flex items-center gap-2">
            <span class="inline-block h-2 w-2 rounded-full bg-ok" />
            <span>Observed {{ formattedObservedAt }} ({{ relativeObservedAt }})</span>
          </div>
          <span>Configuration is restart-only</span>
        </div>

        <!-- Capacity & Resources section -->
        <section class="space-y-3">
          <div>
            <h2 class="text-sm font-semibold text-ink-strong">
              Resources & Capacity
            </h2>
            <p class="mt-0.5 text-xs text-ink-muted">
              Current count of entities managed across this instance.
            </p>
          </div>

          <div class="grid grid-cols-2 gap-3 sm:grid-cols-4">
            <div
              v-for="item in [
                { label: 'Projects', value: status.projects, hint: 'Active projects' },
                { label: 'Environments', value: status.environments, hint: 'Project environments' },
                { label: 'Secrets', value: status.secrets, hint: 'Encrypted items' },
                { label: 'Backups', value: status.backups, hint: 'Database snapshots' },
                { label: 'Users', value: status.humanUsers, hint: 'Human accounts' },
                { label: 'AI agents', value: status.aiAgents, hint: 'Service identities' },
                { label: 'Runner tokens', value: status.activeRunnerTokens, hint: 'CI/CD credentials' },
                { label: 'Agent tokens', value: status.activeAgentTokens, hint: 'API credentials' },
              ]"
              :key="item.label"
              class="rounded-card border border-line bg-panel p-4 transition-colors hover:border-line-strong/60">
              <p class="text-xs text-ink-muted">{{ item.label }}</p>
              <p class="mt-1 font-mono text-2xl font-semibold tracking-tight text-ink-strong">
                {{ item.value }}
              </p>
              <p class="mt-0.5 text-[11px] text-ink-muted/80">{{ item.hint }}</p>
            </div>
          </div>
        </section>
      </div>

      <!-- Unavailable empty state -->
      <div
        v-else
        class="flex flex-col items-center gap-3 py-16 text-ink-muted">
        <ServerIcon class="h-6 w-6" />
        <p class="text-sm">Status unavailable.</p>
      </div>

      <!-- Danger zone for root user -->
      <section
        v-if="isRoot"
        class="mt-10 rounded-card border border-crit/40 bg-crit/5 p-5">
        <h2 class="text-sm font-semibold text-crit">Danger Zone</h2>
        <p class="mt-2 text-sm text-ink-muted">
          Factory reset permanently deletes all projects, secrets, users, sessions, audit history, and server backups. It also deletes the root account and returns this instance to first-install setup.
        </p>
        <p class="mt-2 text-xs text-ink-muted">
          Server configuration and the master key remain. Downloaded backups are not affected.
        </p>
        <DbButton
          class="mt-4"
          variant="danger"
          @click="loadResetPreview">
          Review factory reset
        </DbButton>
        <div
          v-if="resetPreview"
          class="mt-4 space-y-3 border-t border-crit/20 pt-4">
          <p class="font-mono text-xs text-ink-muted">
            Users {{ resetPreview.users }} · AI agents {{ resetPreview.aiAgents }} · Projects {{ resetPreview.projects }} · Environments {{ resetPreview.environments }} · Secrets {{ resetPreview.secrets }} · Backups {{ resetPreview.backups }}
          </p>
          <label class="flex items-center gap-2 text-sm">
            <input v-model="resetAcknowledged" type="checkbox" />
            I understand this permanently deletes the instance data.
          </label>
          <input
            v-model="resetConfirmation"
            class="w-full rounded-control border border-line bg-panel px-3 py-2 font-mono text-sm"
            placeholder="Type FACTORY RESET" />
          <input
            v-model="resetPassword"
            class="w-full rounded-control border border-line bg-panel px-3 py-2 text-sm"
            type="password"
            placeholder="Root password" />
          <p v-if="resetError" class="text-sm text-crit">{{ resetError }}</p>
          <p v-if="resetComplete" class="text-sm text-ink-strong">
            Reset complete. The server is ready for setup again.
          </p>
          <DbButton
            variant="danger"
            :loading="resetLoading"
            :disabled="
              !resetAcknowledged ||
              resetConfirmation !== 'FACTORY RESET' ||
              !resetPassword
            "
            @click="performReset">
            Permanently reset instance
          </DbButton>
        </div>
      </section>
    </div>
  </DashboardLayout>
</template>
