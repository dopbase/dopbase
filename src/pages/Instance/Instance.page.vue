<script setup lang="ts">
import { useInstanceController } from "./Instance.controller";
import { DashboardLayout } from "~/layouts";
import { DbAlert, DbBadge, DbButton, DbSpinner } from "~/components/ui";
import { RefreshIcon, ServerIcon } from "~/assets/icons";

/**
 * Instance — read-only server status: version, endpoint, database health,
 * master-key availability, and the restart-only configuration notice.
 */
const { status, loading, loadError, load, healthTone } =
  useInstanceController();
</script>

<template>
  <DashboardLayout>
    <div class="mx-auto max-w-3xl p-8">
      <header class="mb-6 flex items-center gap-2">
        <div>
          <h1 class="text-lg font-semibold">Instance</h1>
          <p class="mt-0.5 text-sm text-ink-muted">
            Read-only status of this self-hosted server.
          </p>
        </div>
        <DbButton
          class="ml-auto"
          size="sm"
          variant="secondary"
          :loading="loading"
          @click="load">
          <RefreshIcon class="h-3.5 w-3.5" />
          Refresh
        </DbButton>
      </header>

      <DbAlert v-if="loadError" class="mb-4">{{ loadError }}</DbAlert>

      <DbSpinner
        v-if="loading && !status"
        class="mx-auto mt-12 h-6 w-6 text-ink-muted" />

      <div
        v-else-if="status"
        class="grid grid-cols-1 gap-3 sm:grid-cols-2"
        data-testid="instance-cards">
        <div
          class="rounded-[var(--radius-card)] border border-line bg-panel px-4 py-3">
          <p class="text-xs uppercase tracking-wide text-ink-muted">Version</p>
          <p class="mt-1 font-mono text-sm text-ink-strong">
            v{{ status.version }}
          </p>
        </div>
        <div
          class="rounded-[var(--radius-card)] border border-line bg-panel px-4 py-3">
          <p class="text-xs uppercase tracking-wide text-ink-muted">Endpoint</p>
          <p class="mt-1 truncate font-mono text-sm text-ink-strong">
            {{ status.publicUrl }}
          </p>
        </div>
        <div
          class="rounded-[var(--radius-card)] border border-line bg-panel px-4 py-3">
          <p class="text-xs uppercase tracking-wide text-ink-muted">Database</p>
          <p class="mt-1 flex items-center gap-2">
            <DbBadge :tone="healthTone(status.databaseHealth)">
              {{ status.databaseHealth }}
            </DbBadge>
          </p>
        </div>
        <div
          class="rounded-[var(--radius-card)] border border-line bg-panel px-4 py-3">
          <p class="text-xs uppercase tracking-wide text-ink-muted">
            Master key
          </p>
          <p class="mt-1 flex items-center gap-2">
            <DbBadge :tone="healthTone(status.keyAvailability)">
              {{ status.keyAvailability }}
            </DbBadge>
          </p>
        </div>
        <div
          class="rounded-[var(--radius-card)] border border-line bg-panel px-4 py-3 sm:col-span-2">
          <p class="text-xs uppercase tracking-wide text-ink-muted">
            Configuration
          </p>
          <div class="mt-1 flex flex-wrap items-center gap-2">
            <DbBadge tone="warn">
              {{
                status.configurationReload === "restart-required"
                  ? "restart required for changes"
                  : status.configurationReload
              }}
            </DbBadge>
            <span class="text-xs text-ink-muted">
              Server settings load once at startup. Edit
              <code class="font-mono text-xs">~/.dopbase/server.toml</code>
              on the host and restart the process — the browser cannot change
              them. See
              <a
                class="text-accent-strong underline-offset-2 hover:underline"
                href="https://dopbase.dev/self-hosting/operations"
                target="_blank"
                rel="noreferrer">
                operations docs </a
              >.
            </span>
          </div>
        </div>
      </div>

      <div v-else class="flex flex-col items-center gap-3 py-16 text-ink-muted">
        <ServerIcon class="h-6 w-6" />
        <p class="text-sm">Status unavailable.</p>
      </div>
    </div>
  </DashboardLayout>
</template>
