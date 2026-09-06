<script setup lang="ts">
import { DashboardLayout } from "~/layouts";
import { DbAlert, DbBadge, DbButton, DbSelect } from "~/components/ui";
import { useImportSecretsController } from "./ImportSecrets.controller";

/**
 * ImportSecrets — full-page review of a parsed `.env` import.
 *
 * The upload dialog parses the file and hands it over via the import
 * store; this page renders the review at full height so large files
 * scroll freely (the popup version could not). Keys are listed but values
 * are never rendered. Landing here without a pending import for this
 * environment (direct URL, reload) bounces back to the environment.
 */
const {
  environmentId,
  projectRef,
  fileName,
  keys,
  parseErrors,
  stage,
  mode,
  working,
  actionError,
  effectGroups,
  backToEnvironment,
  cancel,
  validate,
  apply,
} = useImportSecretsController();
</script>

<template>
  <DashboardLayout>
    <section class="mx-auto flex w-full max-w-3xl flex-col gap-4 p-6">
      <!-- Header -->
      <header class="flex flex-col gap-1">
        <nav
          class="flex items-center gap-1.5 font-mono text-sm text-ink-muted"
          aria-label="Breadcrumb">
          <button
            type="button"
            class="cursor-pointer transition-colors hover:text-ink-strong"
            @click="backToEnvironment">
            {{ projectRef }}
          </button>
          <span class="text-ink-faint">/</span>
          <button
            type="button"
            class="cursor-pointer font-mono transition-colors hover:text-ink-strong"
            @click="backToEnvironment">
            {{ environmentId }}
          </button>
          <span class="text-ink-faint">/</span>
          <span class="text-ink-strong">import</span>
        </nav>
        <h1 class="text-lg font-semibold">Review import</h1>
        <p class="text-sm text-ink">
          <span class="font-mono text-ink-strong">{{ fileName }}</span> ·
          <span class="font-mono text-ink-strong">{{ keys.length }}</span>
          keys parsed. Values are hidden.
        </p>
      </header>

      <!-- Review: full-height key list -->
      <template v-if="stage === 'review'">
        <div class="rounded-card border border-line bg-panel p-4">
          <p
            class="mb-2 font-mono text-xs uppercase tracking-wide text-ink-faint">
            keys ({{ keys.length }})
          </p>
          <div
            class="max-h-[60vh] overflow-y-auto rounded-control border border-line-soft bg-canvas px-3 py-2">
            <div class="grid grid-cols-1 gap-x-6 sm:grid-cols-2 lg:grid-cols-3">
              <p
                v-for="key in keys"
                :key="key"
                class="truncate border-b border-line-soft/50 py-1 font-mono text-xs text-ink last:border-b-0 sm:border-b-0">
                {{ key }}
              </p>
            </div>
          </div>
        </div>

        <DbSelect
          v-model="mode"
          label="Mode"
          :options="[
            {
              label: 'Merge - add new keys, update existing',
              value: 'merge',
            },
            {
              label: 'Replace - also remove keys not in the file',
              value: 'replace',
            },
          ]" />

        <DbAlert v-if="parseErrors.length > 0" tone="info">
          {{ parseErrors.length }} line(s) were skipped:
          {{ parseErrors.join(" ") }}
        </DbAlert>

        <div class="flex items-center justify-end gap-2">
          <DbButton variant="ghost" :disabled="working" @click="cancel">
            Cancel
          </DbButton>
          <DbButton variant="primary" :loading="working" @click="validate">
            Validate
          </DbButton>
        </div>
      </template>

      <!-- Dry-run result -->
      <template v-else>
        <div class="flex items-center gap-2">
          <DbBadge tone="accent">Validate changes ok</DbBadge>
          <span class="text-xs text-ink-muted">
            mode: {{ mode }} · nothing stored yet
          </span>
        </div>
        <div
          v-for="group in effectGroups"
          :key="group.label"
          class="rounded-card border border-line bg-panel p-4">
          <p
            class="mb-1 font-mono text-xs uppercase tracking-wide text-ink-faint">
            {{ group.label }} ({{ group.keys.length }})
          </p>
          <div class="max-h-56 overflow-y-auto">
            <p class="font-mono text-xs text-ink">
              {{ group.keys.join(", ") }}
            </p>
          </div>
        </div>
        <DbAlert v-if="actionError">{{ actionError }}</DbAlert>
        <div class="flex items-center justify-end gap-2">
          <DbButton
            variant="ghost"
            :disabled="working"
            @click="stage = 'review'">
            Back
          </DbButton>
          <DbButton variant="primary" :loading="working" @click="apply">
            Apply import
          </DbButton>
        </div>
      </template>
    </section>
  </DashboardLayout>
</template>
