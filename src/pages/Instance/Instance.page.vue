<script setup lang="ts">
import { RESET_PHRASE, useInstanceController } from "./Instance.controller";
import type { FixAction, Tone } from "./Instance.controller";
import { DashboardLayout } from "~/layouts";
import PageHeader from "~/components/app/PageHeader.vue";
import {
  DbAlert,
  DbBadge,
  DbButton,
  DbCheckbox,
  DbInfoTip,
  DbInput,
  DbSkeleton,
} from "~/components/ui";
import {
  ActivityIcon,
  AlertTriangleIcon,
  ArchiveIcon,
  CheckIcon,
  ChevronDownIcon,
  CopyIcon,
  DownloadIcon,
  ExternalLinkIcon,
  HistoryIcon,
  KeyIcon,
  RefreshIcon,
  ServerIcon,
  ShieldIcon,
  UsersIcon,
} from "~/assets/icons";

/**
 * Status: what needs attention on this server, the readings behind the
 * health cards, the last day of activity and what it stores. Root also gets
 * the master key download and the factory reset.
 */
const {
  status,
  loading,
  loadError,
  staleSince,
  running,
  runError,
  runChecks,
  hasChecks,
  checks,
  attention,
  passed,
  showPassed,
  banner,
  serverLine,
  releaseBadge,
  healthCards,
  activityTiles,
  fixAction,
  runFix,
  severityLabel,
  severityTone,
  diagnosticsCopied,
  copyDiagnostics,
  counts,
  isRoot,
  downloadingMasterKey,
  masterKeyMessage,
  resetOpen,
  resetPassword,
  resetConfirmation,
  resetAcknowledged,
  resetLoading,
  resetError,
  resetComplete,
  canReset,
  performReset,
} = useInstanceController();

const FIX_ICONS: Record<FixAction["icon"], unknown> = {
  archive: ArchiveIcon,
  download: DownloadIcon,
  key: KeyIcon,
  external: ExternalLinkIcon,
  audit: HistoryIcon,
  users: UsersIcon,
};

const CARD_ICONS = {
  database: ServerIcon,
  masterKey: KeyIcon,
  disk: ArchiveIcon,
  auditLog: ShieldIcon,
};

const BANNER_BORDERS: Record<Tone, string> = {
  ok: "border-ok/50",
  warn: "border-warn/60",
  crit: "border-crit/60",
  neutral: "border-line",
  accent: "border-accent/50",
};

const BANNER_TILES: Record<Tone, string> = {
  ok: "bg-ok/15 text-ok",
  warn: "bg-warn/15 text-warn",
  crit: "bg-crit/15 text-crit",
  neutral: "bg-raised text-ink",
  accent: "bg-accent/15 text-accent-strong",
};

/** Bars for a 24-hour sparkline. Empty hours stay as a faint stub. */
function bars(hourly: number[]): { height: number; empty: boolean }[] {
  const peak = Math.max(...hourly, 1);
  return hourly.map((count) => ({
    height: Math.max(12, Math.round((count / peak) * 100)),
    empty: count === 0,
  }));
}

function formatTime(iso: string): string {
  return new Date(iso).toLocaleTimeString([], {
    hour: "2-digit",
    minute: "2-digit",
  });
}
</script>

<template>
  <DashboardLayout>
    <div class="page-width flex flex-col gap-5 py-6 lg:py-8">
      <PageHeader
        title="Status"
        subtitle="Checks run every 30 seconds while this page is open.">
        <template #actions>
          <DbButton
            :disabled="!status"
            data-testid="copy-diagnostics"
            @click="copyDiagnostics">
            <CheckIcon v-if="diagnosticsCopied" class="h-4 w-4" />
            <CopyIcon v-else class="h-4 w-4" />
            {{ diagnosticsCopied ? "Copied" : "Copy diagnostics" }}
          </DbButton>
          <DbButton
            v-if="hasChecks"
            variant="primary"
            :loading="running"
            data-testid="run-checks"
            @click="runChecks">
            <RefreshIcon class="h-4 w-4" />
            Run checks
          </DbButton>
        </template>
      </PageHeader>

      <DbAlert v-if="loadError">{{ loadError }}</DbAlert>
      <DbAlert v-if="runError" data-testid="run-error">{{ runError }}</DbAlert>
      <DbAlert v-if="staleSince" tone="info" data-testid="stale-status">
        Couldn't refresh. These results are from
        {{ formatTime(staleSince) }}.
      </DbAlert>
      <DbAlert
        v-if="masterKeyMessage"
        :tone="masterKeyMessage.tone === 'ok' ? 'success' : 'error'"
        data-testid="master-key-message">
        {{ masterKeyMessage.text }}
      </DbAlert>

      <!-- First load -->
      <div
        v-if="loading && !status"
        class="flex flex-col gap-5"
        data-testid="instance-skeleton">
        <DbSkeleton class="h-[86px] w-full rounded-card" />
        <div class="grid grid-cols-1 gap-4 lg:grid-cols-2">
          <div class="grid grid-cols-2 gap-4">
            <DbSkeleton v-for="i in 4" :key="i" class="h-32 rounded-card" />
          </div>
          <DbSkeleton class="h-[280px] rounded-card" />
        </div>
        <DbSkeleton class="h-52 w-full rounded-card" />
      </div>

      <template v-else-if="status">
        <!-- Banner -->
        <section
          v-if="banner"
          class="flex flex-wrap items-center gap-x-3.5 gap-y-3 rounded-card border bg-panel px-5 py-4"
          :class="BANNER_BORDERS[banner.tone]"
          :data-tone="banner.tone"
          data-testid="status-banner">
          <div class="flex min-w-0 flex-1 basis-80 items-center gap-3.5">
            <span
              class="flex h-9 w-9 shrink-0 items-center justify-center rounded-control"
              :class="BANNER_TILES[banner.tone]">
              <CheckIcon
                v-if="banner.tone === 'ok'"
                class="h-[18px] w-[18px]" />
              <AlertTriangleIcon v-else class="h-[18px] w-[18px]" />
            </span>
            <div class="min-w-0 flex-1">
              <p class="text-[15px] font-semibold text-ink-strong">
                {{ banner.title }}
              </p>
              <p class="text-sm text-ink-muted">{{ banner.detail }}</p>
            </div>
          </div>
          <a
            v-if="banner.keyProblem"
            href="https://docs.dopbase.com/self-hosting/encryption-keys"
            target="_blank"
            rel="noopener"
            class="inline-flex h-8 items-center gap-1.5 rounded-control bg-raised px-3 text-xs font-medium text-ink-strong hover:bg-line">
            <ExternalLinkIcon class="h-3.5 w-3.5" />
            How to fix it
          </a>
          <div v-else class="flex flex-col items-start gap-1.5 sm:items-end">
            <p class="font-mono text-sm text-ink-muted">{{ serverLine }}</p>
            <DbBadge
              v-if="releaseBadge"
              :tone="releaseBadge.tone"
              data-testid="release-badge">
              {{ releaseBadge.text }}
            </DbBadge>
          </div>
        </section>

        <!-- Health and the last 24 hours -->
        <div
          v-if="healthCards.length"
          class="grid grid-cols-1 gap-4 lg:grid-cols-2"
          data-testid="health-row">
          <div class="grid grid-cols-1 gap-4 sm:grid-cols-2">
            <div
              v-for="card in healthCards"
              :key="card.id"
              class="flex min-w-0 flex-col gap-1.5 rounded-card border border-line bg-panel px-4 py-3.5"
              :data-testid="`health-${card.id}`">
              <p class="flex items-center gap-1.5 text-sm text-ink-muted">
                <component
                  :is="CARD_ICONS[card.id]"
                  class="h-3.5 w-3.5 shrink-0" />
                {{ card.label }}
              </p>
              <p class="flex flex-wrap items-center gap-2">
                <span class="text-lg font-semibold text-ink-strong">
                  {{ card.value }}
                </span>
                <DbBadge :tone="card.badge.tone">{{ card.badge.text }}</DbBadge>
              </p>
              <p class="text-sm text-ink-muted">{{ card.detail }}</p>
            </div>
          </div>

          <section
            v-if="activityTiles.length"
            class="flex flex-col gap-3.5 rounded-card border border-line bg-panel px-5 py-[18px]"
            data-testid="activity">
            <div class="flex items-center justify-between gap-3">
              <h2 class="text-[15px] font-semibold text-ink-strong">
                Last 24 hours
              </h2>
              <RouterLink
                to="/audit"
                class="inline-flex h-8 items-center gap-1.5 rounded-control px-3 text-xs font-medium text-ink hover:bg-raised hover:text-ink-strong">
                Audit log
              </RouterLink>
            </div>
            <dl class="grid flex-1 grid-cols-1 gap-2 sm:grid-cols-2">
              <div
                v-for="tile in activityTiles"
                :key="tile.id"
                class="flex min-w-0 flex-col gap-1 rounded-control bg-editor px-3 py-2.5"
                :data-testid="`activity-${tile.id}`">
                <dt class="text-sm text-ink-muted">{{ tile.label }}</dt>
                <dd class="flex items-end gap-3">
                  <span
                    class="font-mono text-sm font-semibold"
                    :class="
                      tile.tone === 'warn' ? 'text-warn' : 'text-ink-strong'
                    ">
                    {{ tile.value }}
                  </span>
                  <span
                    v-if="tile.hourly"
                    class="flex h-4 flex-1 items-end gap-px"
                    aria-hidden="true">
                    <span
                      v-for="(bar, index) in bars(tile.hourly)"
                      :key="index"
                      class="w-1 rounded-[1px]"
                      :class="bar.empty ? 'bg-line-strong' : 'bg-accent'"
                      :style="{ height: `${bar.height}%` }" />
                  </span>
                </dd>
                <dd
                  class="truncate text-xs text-ink-muted"
                  :title="tile.detail">
                  {{ tile.detail }}
                </dd>
              </div>
            </dl>
          </section>
        </div>

        <!-- Failing checks, or every check when all pass -->
        <section
          v-if="hasChecks"
          class="flex flex-col rounded-card border border-line bg-panel px-5 py-[18px]"
          data-testid="checks">
          <div class="pb-2.5">
            <h2 class="text-[15px] font-semibold text-ink-strong">
              {{ attention.length ? "Needs attention" : "Checks" }}
            </h2>
            <p class="text-sm text-ink-muted">
              {{
                attention.length
                  ? "Most important first."
                  : "Every check that runs, and what it saw last time."
              }}
            </p>
          </div>

          <div
            v-for="check in attention"
            :key="check.id"
            class="flex flex-wrap items-center gap-x-3.5 gap-y-2 border-t border-line-soft py-3.5"
            :data-testid="`check-${check.id}`">
            <span class="w-[88px] shrink-0">
              <DbBadge :tone="severityTone(check.status)">
                {{ severityLabel(check.status) }}
              </DbBadge>
            </span>
            <div class="min-w-0 flex-1 basis-60">
              <p class="text-sm font-semibold text-ink-strong">
                {{ check.title }}
              </p>
              <p class="text-sm text-ink-muted">{{ check.detail }}</p>
            </div>
            <DbButton
              v-if="fixAction(check)"
              size="sm"
              :loading="check.id === 'master_key.file' && downloadingMasterKey"
              :data-testid="`fix-${check.id}`"
              @click="runFix(check)">
              <component
                :is="FIX_ICONS[fixAction(check)!.icon]"
                class="h-3.5 w-3.5" />
              {{ fixAction(check)!.label }}
            </DbButton>
          </div>

          <template v-if="attention.length">
            <div
              v-if="passed.length"
              class="flex items-center gap-2 border-t border-line-soft pt-2.5 text-sm text-ink-muted">
              <CheckIcon class="h-3.5 w-3.5 text-ok" />
              <span class="flex-1">
                {{ passed.length }}
                {{ passed.length === 1 ? "check" : "checks" }} passed
              </span>
              <DbButton
                variant="ghost"
                size="sm"
                data-testid="toggle-passed"
                @click="showPassed = !showPassed">
                <ChevronDownIcon
                  class="h-3.5 w-3.5 transition-transform"
                  :class="showPassed ? 'rotate-180' : ''" />
                {{ showPassed ? "Hide" : "Show all" }}
              </DbButton>
            </div>
          </template>

          <template v-if="!attention.length || showPassed">
            <div
              v-for="check in passed"
              :key="check.id"
              class="flex items-center gap-2.5 border-t border-line-soft py-2.5"
              :data-testid="`passed-${check.id}`">
              <CheckIcon class="h-3.5 w-3.5 shrink-0 text-ok" />
              <span
                class="min-w-0 flex-1 text-sm text-ink"
                :title="check.detail">
                {{ check.title }}
              </span>
              <span
                v-if="check.value"
                class="max-w-[45%] truncate font-mono text-sm text-ink-muted">
                {{ check.value }}
              </span>
            </div>
          </template>

          <p v-if="!checks.length" class="py-4 text-sm text-ink-muted">
            No checks have run yet.
          </p>
        </section>

        <section
          class="flex flex-col gap-3.5 rounded-card border border-line bg-panel p-4 sm:p-5"
          data-testid="instance-content">
          <div>
            <h2 class="text-[15px] font-semibold text-ink-strong">
              What's stored
            </h2>
            <p class="text-sm text-ink-muted">
              Counts across the whole instance.
            </p>
          </div>
          <dl class="grid grid-cols-2 gap-3 sm:grid-cols-3 lg:grid-cols-4">
            <div
              v-for="item in counts"
              :key="item.label"
              class="flex min-w-0 flex-col gap-0.5 rounded-control bg-editor px-3.5 py-2.5 text-sm">
              <dt class="flex min-w-0 items-center gap-1.5 text-ink-muted">
                <span class="truncate">{{ item.label }}</span>
                <DbInfoTip
                  v-if="item.hint"
                  :text="item.hint"
                  :label="`About ${item.label}`" />
              </dt>
              <dd class="font-mono font-semibold text-ink-strong">
                {{ item.value }}
              </dd>
            </div>
          </dl>
        </section>
      </template>

      <div
        v-else-if="!loadError"
        class="flex flex-col items-center gap-3 py-16 text-ink-muted">
        <ActivityIcon class="h-6 w-6" />
        <p class="text-sm">Status unavailable.</p>
      </div>

      <section
        v-if="isRoot"
        class="flex flex-col gap-3.5 rounded-card border border-crit/60 bg-panel p-4 sm:p-5"
        data-testid="factory-reset">
        <div class="flex flex-wrap items-center gap-x-4 gap-y-3">
          <div class="min-w-0 flex-1 basis-72">
            <h2 class="text-[15px] font-semibold text-crit">Factory reset</h2>
            <p class="text-sm text-ink-muted">
              Deletes everything counted above, plus sessions and the audit log,
              and sends the server back to first-run setup. Only root sees this
              card.
            </p>
          </div>
          <DbButton
            size="sm"
            :aria-expanded="resetOpen"
            data-testid="toggle-reset"
            @click="resetOpen = !resetOpen">
            <ChevronDownIcon
              class="h-3.5 w-3.5 transition-transform"
              :class="resetOpen ? 'rotate-180' : ''" />
            {{ resetOpen ? "Hide reset steps" : "Show reset steps" }}
          </DbButton>
        </div>
        <template v-if="resetOpen">
          <p class="text-sm text-ink-muted">
            Configuration and the master key stay, and downloaded backups aren't
            touched.
          </p>
          <DbCheckbox
            v-model="resetAcknowledged"
            label="I've downloaded a backup and the master key" />
          <div class="grid grid-cols-1 gap-3 sm:grid-cols-2">
            <DbInput
              v-model="resetConfirmation"
              :label="`Type ${RESET_PHRASE}`"
              name="resetConfirmation"
              autocomplete="off"
              mono />
            <DbInput
              v-model="resetPassword"
              label="Root password"
              name="resetPassword"
              type="password"
              autocomplete="current-password" />
          </div>
          <DbAlert v-if="resetError">{{ resetError }}</DbAlert>
          <DbAlert v-if="resetComplete" tone="success">
            Reset done. The server is ready for setup again.
          </DbAlert>
          <div class="flex justify-end">
            <DbButton
              variant="danger"
              :loading="resetLoading"
              :disabled="!canReset"
              data-testid="factory-reset-submit"
              @click="performReset">
              Reset this instance
            </DbButton>
          </div>
        </template>
      </section>
    </div>
  </DashboardLayout>
</template>
