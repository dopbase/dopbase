<script setup lang="ts">
import { RESET_PHRASE, useInstanceController } from "./Instance.controller";
import { DashboardLayout } from "~/layouts";
import PageHeader from "~/components/app/PageHeader.vue";
import {
  DbAlert,
  DbBadge,
  DbButton,
  DbCheckbox,
  DbInput,
  DbSkeleton,
} from "~/components/ui";
import {
  BoxIcon,
  ClockIcon,
  DownloadIcon,
  KeyIcon,
  RefreshIcon,
  ServerIcon,
} from "~/assets/icons";

/**
 * Status: the server's version, storage and master key health, uptime and
 * what it stores. Root also gets the master key download and the factory
 * reset card.
 */
const {
  status,
  loading,
  loadError,
  load,
  healthTone,
  healthLabel,
  healthBadge,
  counts,
  formattedUptime,
  formattedObservedAt,
  relativeBootTime,
  isRoot,
  downloadingMasterKey,
  masterKeyMessage,
  downloadMasterKey,
  resetPassword,
  resetConfirmation,
  resetAcknowledged,
  resetLoading,
  resetError,
  resetComplete,
  canReset,
  performReset,
} = useInstanceController();
</script>

<template>
  <DashboardLayout>
    <div class="page-width flex flex-col gap-5 py-6 lg:py-8">
      <PageHeader
        title="Status"
        subtitle="Health of this server. Configuration loads at startup, so changes need a restart.">
        <template #actions>
          <DbButton :loading="loading" @click="load">
            <RefreshIcon class="h-4 w-4" />
            Refresh
          </DbButton>
        </template>
      </PageHeader>

      <DbAlert v-if="loadError">{{ loadError }}</DbAlert>
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
        <div class="grid grid-cols-2 gap-3 lg:grid-cols-4 lg:gap-4">
          <div
            v-for="i in 4"
            :key="i"
            class="flex flex-col gap-2.5 rounded-card border border-line bg-panel px-4 py-3.5">
            <DbSkeleton class="h-4 w-20" />
            <DbSkeleton class="h-6 w-24" />
          </div>
        </div>
        <DbSkeleton class="h-52 w-full rounded-card" />
      </div>

      <template v-else-if="status">
        <div
          class="grid grid-cols-2 gap-3 lg:grid-cols-4 lg:gap-4"
          data-testid="instance-cards">
          <div
            class="flex min-w-0 flex-col gap-1.5 rounded-card border border-line bg-panel px-4 py-3.5">
            <p class="flex items-center gap-1.5 text-sm text-ink-muted">
              <BoxIcon class="h-3.5 w-3.5 shrink-0" />
              Version
            </p>
            <p
              class="truncate text-lg font-semibold text-ink-strong"
              :title="`Seen ${formattedObservedAt}`">
              {{ status.version }}
            </p>
          </div>

          <div
            class="flex min-w-0 flex-col gap-1.5 rounded-card border border-line bg-panel px-4 py-3.5">
            <p class="flex items-center gap-1.5 text-sm text-ink-muted">
              <ServerIcon class="h-3.5 w-3.5 shrink-0" />
              Storage
            </p>
            <p class="flex flex-wrap items-center gap-2">
              <span class="text-lg font-semibold text-ink-strong">
                {{ healthLabel(status.databaseHealth) }}
              </span>
              <DbBadge :tone="healthTone(status.databaseHealth)">
                {{ healthBadge(status.databaseHealth) }}
              </DbBadge>
            </p>
          </div>

          <div
            class="flex min-w-0 flex-col gap-1.5 rounded-card border border-line bg-panel px-4 py-3.5"
            data-testid="master-key-card">
            <p class="flex items-center gap-1.5 text-sm text-ink-muted">
              <KeyIcon class="h-3.5 w-3.5 shrink-0" />
              Master key
            </p>
            <p class="flex flex-wrap items-center gap-2">
              <span class="text-lg font-semibold text-ink-strong">
                {{ healthLabel(status.keyAvailability) }}
              </span>
              <DbBadge :tone="healthTone(status.keyAvailability)">
                {{ healthBadge(status.keyAvailability) }}
              </DbBadge>
            </p>
            <DbButton
              v-if="isRoot"
              class="mt-1 self-start"
              size="sm"
              :loading="downloadingMasterKey"
              data-testid="download-master-key"
              @click="downloadMasterKey">
              <DownloadIcon class="h-3.5 w-3.5" />
              Download
            </DbButton>
          </div>

          <div
            class="flex min-w-0 flex-col gap-1.5 rounded-card border border-line bg-panel px-4 py-3.5">
            <p class="flex items-center gap-1.5 text-sm text-ink-muted">
              <ClockIcon class="h-3.5 w-3.5 shrink-0" />
              Uptime
            </p>
            <p
              class="truncate text-lg font-semibold text-ink-strong"
              :title="`Started ${relativeBootTime}`">
              {{ formattedUptime }}
            </p>
          </div>
        </div>

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
              <dt class="truncate text-ink-muted">{{ item.label }}</dt>
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
        <ServerIcon class="h-6 w-6" />
        <p class="text-sm">Status unavailable.</p>
      </div>

      <section
        v-if="isRoot"
        class="flex flex-col gap-3.5 rounded-card border border-crit/60 bg-panel p-4 sm:p-5"
        data-testid="factory-reset">
        <div>
          <h2 class="text-[15px] font-semibold text-crit">Factory reset</h2>
          <p class="text-sm text-ink-muted">
            Deletes everything counted above, plus sessions and the audit log.
            The server goes back to first-run setup. Configuration and the
            master key stay, and downloaded backups aren't touched. Only root
            sees this card.
          </p>
        </div>
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
      </section>
    </div>
  </DashboardLayout>
</template>
