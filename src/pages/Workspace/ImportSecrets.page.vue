<script setup lang="ts">
import { computed, onUnmounted, ref } from "vue";
import { useRoute, useRouter } from "vue-router";
import { DashboardLayout } from "~/layouts";
import { DbAlert, DbBadge, DbButton, DbSelect } from "~/components/ui";
import * as secretsApi from "~/services/secrets.api";
import type { ImportMode, ImportSecretsResponse } from "~/services/secrets.api";
import { useImportStore } from "~/stores/import.store";

/**
 * ImportSecrets — full-page review of a parsed `.env` import.
 *
 * The upload dialog parses the file and hands it over via the import
 * store; this page renders the review at full height so large files
 * scroll freely (the popup version could not). Keys are listed but values
 * are never rendered. Landing here without a pending import for this
 * environment (direct URL, reload) bounces back to the environment.
 */
const route = useRoute();
const router = useRouter();
const importStore = useImportStore();

const environmentId = computed(() =>
  typeof route.params.environmentId === "string"
    ? route.params.environmentId
    : null,
);
const projectRef = computed(() =>
  typeof route.params.projectRef === "string" ? route.params.projectRef : null,
);

/** The pending import must exist and target this route's environment. */
const valid = computed(
  () =>
    environmentId.value !== null &&
    importStore.pending !== null &&
    importStore.pending.environmentId === environmentId.value &&
    importStore.pending.entries.length > 0,
);

if (!valid.value) {
  void router.replace({ name: "environment", params: route.params });
}

// Leaving the page (apply, cancel, or navigating away) drops the pending
// import — it lives only in memory.
onUnmounted(() => importStore.clear());

const fileName = computed(() => importStore.pending?.fileName ?? "");
const keys = computed(
  () => importStore.pending?.entries.map((e) => e.key) ?? [],
);
const parseErrors = computed(() => importStore.pending?.errors ?? []);

const stage = ref<"review" | "dry">("review");
const mode = ref<ImportMode>("merge");
const working = ref(false);
const actionError = ref<string | null>(null);
const dryResult = ref<ImportSecretsResponse | null>(null);

const effectGroups = computed(() => {
  const result = dryResult.value;
  if (!result) return [];
  return [
    { label: "added", keys: result.addedKeys },
    { label: "updated", keys: result.updatedKeys },
    { label: "unchanged", keys: result.unchangedKeys },
    { label: "deleted", keys: result.deletedKeys },
  ].filter((group) => group.keys.length > 0);
});

function backToEnvironment(): void {
  void router.push({ name: "environment", params: route.params });
}

function cancel(): void {
  backToEnvironment();
}

async function validate(): Promise<void> {
  if (importStore.pending === null || environmentId.value === null) return;
  working.value = true;
  actionError.value = null;
  try {
    dryResult.value = await secretsApi.importSecrets(environmentId.value, {
      mode: mode.value,
      dryRun: true,
      entries: importStore.pending.entries,
    });
    stage.value = "dry";
  } catch {
    actionError.value =
      "The import is not valid. Check the file and try again.";
  } finally {
    working.value = false;
  }
}

async function apply(): Promise<void> {
  if (importStore.pending === null || environmentId.value === null) return;
  working.value = true;
  actionError.value = null;
  try {
    await secretsApi.importSecrets(environmentId.value, {
      mode: mode.value,
      dryRun: false,
      entries: importStore.pending.entries,
      expectedRevision: dryResult.value?.revision,
    });
    // The timestamp query makes the secrets table refetch on return; the
    // pending import is cleared by onUnmounted during this navigation.
    void router.push({
      name: "environment",
      params: route.params,
      query: { imported: String(Date.now()) },
    });
  } catch {
    actionError.value =
      "The import failed on the server. Nothing may have changed — re-run the dry run to confirm.";
  } finally {
    working.value = false;
  }
}
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
        <div
          class="rounded-[var(--radius-card)] border border-line bg-panel p-4">
          <p
            class="mb-2 font-mono text-xs uppercase tracking-wide text-ink-faint">
            keys ({{ keys.length }})
          </p>
          <div
            class="max-h-[60vh] overflow-y-auto rounded-md border border-line-soft bg-canvas px-3 py-2">
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
              label: 'Merge — add new keys, update existing',
              value: 'merge',
            },
            {
              label: 'Replace — also remove keys not in the file',
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
          <DbBadge tone="accent">dry-run ok</DbBadge>
          <span class="text-xs text-ink-muted">
            mode: {{ mode }} · nothing stored yet
          </span>
        </div>
        <div
          v-for="group in effectGroups"
          :key="group.label"
          class="rounded-[var(--radius-card)] border border-line bg-panel p-4">
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
