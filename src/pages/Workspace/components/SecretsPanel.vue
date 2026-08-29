<script setup lang="ts">
import { computed, ref } from "vue";
import { useSecretsPanelController } from "./SecretsPanel.controller";
import EnvFileEditor from "./EnvFileEditor.vue";
import ImportSecretsDialog from "./ImportSecretsDialog.vue";
import ExportSecretsDialog from "./ExportSecretsDialog.vue";
import {
  DbAlert,
  DbBadge,
  DbButton,
  DbConfirmDialog,
  DbCopyButton,
  DbEmptyState,
  DbInput,
  DbModal,
  DbSpinner,
  DbTextarea,
} from "~/components/ui";
import {
  EyeIcon,
  EyeOffIcon,
  KeyIcon,
  PencilIcon,
  TrashIcon,
} from "~/assets/icons";
import { formatRelativeTime } from "~/utils/format";
import { ApiError } from "~/services/http.client";
import type { SecretMetadata } from "~/services";

/**
 * SecretsPanel — the secrets table for one environment.
 *
 * Metadata only: values are fetched exclusively via the reveal flow with
 * its 30-second auto-hide, or submitted through the create/edit dialog and
 * immediately discarded. Values are never rendered in listings.
 *
 * The "Edit as .env" view is the exception: it is a reauth-gated, in-memory
 * editing buffer (see the controller) wiped on close, environment change,
 * and unmount.
 */
const props = defineProps<{
  environmentId: string;
  environmentName: string;
  projectName: string;
}>();

const environmentIdRef = computed(() => props.environmentId);
const controller = useSecretsPanelController(environmentIdRef);
// Destructure so refs auto-unwrap in the template.
const {
  secrets,
  loading,
  loadError,
  actionError,
  revealedKey,
  revealedValue,
  revealCountdown,
  hideRevealed,
  reveal,
  editorContent,
  editorLoading,
  editorLoadError,
  editorAwaitingReauth,
  editorSaving,
  editorError,
  editorDiff,
  editorIssues,
  editorEntries,
  editorDirty,
  editorCanSave,
  openEditor,
  closeEditor,
  saveDraft,
  applyDraft,
  discardDraft,
  backToEditing,
} = controller;

type FormState =
  | { mode: "create"; key: string; value: string; error: string | null }
  | {
      mode: "edit";
      secret: SecretMetadata;
      value: string;
      error: string | null;
    }
  | null;

type PanelView = "table" | "editor";

const view = ref<PanelView>("table");
/** Pending view switch while an unsaved-editor discard is confirmed. */
const discardTarget = ref<PanelView | null>(null);

function switchView(next: PanelView): void {
  if (next === view.value) return;
  if (view.value === "editor" && editorDirty.value) {
    discardTarget.value = next;
    return;
  }
  commitSwitch(next);
}

function commitSwitch(next: PanelView): void {
  view.value = next;
  if (next === "editor") void openEditor();
  else closeEditor();
}

function confirmDiscardSwitch(): void {
  const next = discardTarget.value;
  discardTarget.value = null;
  if (next !== null) commitSwitch(next);
}

const diffGroups = computed(() => {
  const diff = editorDiff.value;
  if (!diff) return [];
  return [
    { label: "Added", keys: diff.addedKeys },
    { label: "Updated", keys: diff.updatedKeys },
    { label: "Unchanged", keys: diff.unchangedKeys },
    { label: "Deleted", keys: diff.deletedKeys },
  ].filter((group) => group.keys.length > 0);
});

const form = ref<FormState>(null);
const saving = ref(false);
const deleteTarget = ref<SecretMetadata | null>(null);
const deleteLoading = ref(false);
const deleteError = ref<string | null>(null);
const showImport = ref(false);
const showExport = ref(false);

function openCreate(): void {
  form.value = { mode: "create", key: "", value: "", error: null };
}

function openEdit(secret: SecretMetadata): void {
  form.value = { mode: "edit", secret, value: "", error: null };
}

function closeForm(): void {
  if (!saving.value) form.value = null;
}

async function submitForm(): Promise<void> {
  if (!form.value) return;
  if (form.value.mode === "create" && form.value.key.trim() === "") {
    form.value.error = "Enter a key name.";
    return;
  }
  saving.value = true;
  try {
    const key =
      form.value.mode === "create"
        ? form.value.key.trim()
        : form.value.secret.key;
    await controller.setSecret(key, form.value.value);
    form.value = null;
  } catch (cause) {
    if (form.value) {
      form.value.error =
        cause instanceof ApiError && cause.hasCode("REQUEST_INVALID")
          ? "Keys may use letters, numbers, '_', '-', or '.', and cannot start with a number."
          : "The secret could not be saved.";
    }
  } finally {
    saving.value = false;
  }
}

async function confirmDelete(): Promise<void> {
  if (!deleteTarget.value) return;
  deleteLoading.value = true;
  deleteError.value = null;
  try {
    await controller.deleteSecret(deleteTarget.value.key);
    deleteTarget.value = null;
  } catch {
    deleteError.value = "The secret could not be deleted.";
  } finally {
    deleteLoading.value = false;
  }
}
</script>

<template>
  <div class="flex flex-col gap-4">
    <header class="flex flex-wrap items-center gap-2">
      <h3 class="text-sm font-semibold">
        Secrets
        <DbBadge v-if="secrets" class="ml-1">
          {{ secrets.length }}
        </DbBadge>
      </h3>
      <div class="ml-auto flex items-center gap-2">
        <div
          class="flex rounded-md border border-line bg-canvas p-0.5"
          role="tablist"
          aria-label="Secrets view">
          <button
            type="button"
            role="tab"
            :aria-selected="view === 'table'"
            data-testid="secrets-view-table"
            class="cursor-pointer rounded px-2.5 py-1 text-xs font-medium transition-colors"
            :class="
              view === 'table'
                ? 'bg-raised text-ink-strong'
                : 'text-ink-muted hover:text-ink'
            "
            @click="switchView('table')">
            Table
          </button>
          <button
            type="button"
            role="tab"
            :aria-selected="view === 'editor'"
            data-testid="secrets-view-editor"
            class="cursor-pointer rounded px-2.5 py-1 text-xs font-medium transition-colors"
            :class="
              view === 'editor'
                ? 'bg-raised text-ink-strong'
                : 'text-ink-muted hover:text-ink'
            "
            @click="switchView('editor')">
            Edit as .env
          </button>
        </div>
        <template v-if="view === 'table'">
          <DbButton size="sm" variant="secondary" @click="showExport = true">
            Export
          </DbButton>
          <DbButton size="sm" variant="secondary" @click="showImport = true">
            Import .env
          </DbButton>
          <DbButton size="sm" variant="primary" @click="openCreate">
            Add secret
          </DbButton>
        </template>
      </div>
    </header>

    <!-- Table view -->
    <template v-if="view === 'table'">
      <DbAlert v-if="actionError">
        {{ actionError }}
      </DbAlert>

      <DbSpinner v-if="loading" class="mx-auto mt-8 h-5 w-5 text-ink-muted" />

      <DbAlert v-else-if="loadError">
        {{ loadError }}
      </DbAlert>

      <DbEmptyState
        v-else-if="secrets && secrets.length === 0"
        title="No secrets in this environment"
        description="Add a single key, or import an existing .env file — values are encrypted before they are stored.">
        <template #icon>
          <KeyIcon class="h-5 w-5" />
        </template>
        <template #actions>
          <DbButton variant="secondary" size="sm" @click="showImport = true">
            Import .env
          </DbButton>
          <DbButton variant="primary" size="sm" @click="openCreate">
            Add secret
          </DbButton>
        </template>
      </DbEmptyState>

      <div
        v-else-if="secrets"
        class="overflow-x-auto rounded-[var(--radius-card)] border border-line bg-panel">
        <table class="min-w-full text-left text-sm" data-testid="secrets-table">
          <thead>
            <tr class="border-b border-line">
              <th
                class="px-4 py-2.5 text-xs font-medium uppercase tracking-wide text-ink-muted">
                Key
              </th>
              <th
                class="px-4 py-2.5 text-xs font-medium uppercase tracking-wide text-ink-muted">
                Version
              </th>
              <th
                class="px-4 py-2.5 text-xs font-medium uppercase tracking-wide text-ink-muted">
                Updated
              </th>
              <th
                class="px-4 py-2.5 text-right text-xs font-medium uppercase tracking-wide text-ink-muted">
                Actions
              </th>
            </tr>
          </thead>
          <tbody>
            <template v-for="secret in secrets" :key="secret.key">
              <tr class="border-b border-line-soft last:border-b-0">
                <td class="px-4 py-2.5 font-mono text-sm text-ink-strong">
                  {{ secret.key }}
                </td>
                <td class="px-4 py-2.5">
                  <DbBadge>v{{ secret.version }}</DbBadge>
                </td>
                <td class="px-4 py-2.5 text-sm text-ink-muted">
                  {{ formatRelativeTime(secret.updatedAt) }}
                </td>
                <td class="px-4 py-2.5">
                  <div class="flex items-center justify-end gap-1">
                    <button
                      v-if="revealedKey !== secret.key"
                      type="button"
                      class="cursor-pointer rounded border border-line bg-raised px-2 py-1 font-mono text-xs text-ink transition-colors hover:border-accent/50 hover:text-ink-strong"
                      @click="reveal(secret.key)">
                      <span class="flex items-center gap-1.5">
                        <EyeIcon class="h-3.5 w-3.5" />
                        reveal
                      </span>
                    </button>
                    <button
                      v-else
                      type="button"
                      class="cursor-pointer rounded border border-accent/50 bg-accent-soft px-2 py-1 font-mono text-xs text-accent-strong"
                      @click="hideRevealed()">
                      <span class="flex items-center gap-1.5">
                        <EyeOffIcon class="h-3.5 w-3.5" />
                        hide
                      </span>
                    </button>
                    <button
                      type="button"
                      class="cursor-pointer rounded p-1.5 text-ink-muted transition-colors hover:text-ink-strong"
                      :aria-label="`Edit ${secret.key}`"
                      @click="openEdit(secret)">
                      <PencilIcon class="h-3.5 w-3.5" />
                    </button>
                    <button
                      type="button"
                      class="cursor-pointer rounded p-1.5 text-ink-muted transition-colors hover:text-crit"
                      :aria-label="`Delete ${secret.key}`"
                      @click="deleteTarget = secret">
                      <TrashIcon class="h-3.5 w-3.5" />
                    </button>
                  </div>
                </td>
              </tr>
              <!-- Revealed plaintext row: component memory only, auto-hides -->
              <tr
                v-if="revealedKey === secret.key"
                class="border-b border-line-soft bg-accent-soft/40 last:border-b-0">
                <td colspan="4" class="px-4 py-3">
                  <div class="flex flex-wrap items-center gap-3">
                    <code
                      class="min-w-0 flex-1 break-all rounded border border-accent/30 bg-canvas px-3 py-2 font-mono text-xs text-accent-strong">
                      {{ revealedValue }}
                    </code>
                    <DbCopyButton
                      v-if="revealedValue"
                      :value="revealedValue"
                      label="Copy" />
                    <DbBadge tone="warn">
                      hides in {{ revealCountdown }}
                    </DbBadge>
                  </div>
                </td>
              </tr>
            </template>
          </tbody>
        </table>
      </div>
    </template>

    <!-- Editor view -->
    <template v-else>
      <div class="flex flex-wrap items-center gap-2">
        <span class="text-xs text-ink-muted">
          <span class="font-mono text-ink-strong">{{
            editorEntries.length
          }}</span>
          keys ·
          <span
            class="font-mono"
            :class="editorIssues.length > 0 ? 'text-crit' : 'text-ink-strong'">
            {{ editorIssues.length }}
          </span>
          issues
        </span>
        <DbBadge v-if="editorDirty" tone="warn">unsaved changes</DbBadge>
        <DbBadge v-else-if="editorContent !== null" tone="ok">saved</DbBadge>
        <div class="ml-auto flex items-center gap-2">
          <DbButton
            size="sm"
            variant="ghost"
            :disabled="editorSaving || !editorDirty"
            data-testid="env-editor-revert"
            @click="discardDraft">
            Revert
          </DbButton>
          <DbButton
            size="sm"
            variant="primary"
            :disabled="!editorCanSave"
            :loading="editorSaving"
            data-testid="env-editor-save"
            @click="saveDraft">
            Save
          </DbButton>
        </div>
      </div>

      <DbSpinner
        v-if="editorLoading"
        class="mx-auto mt-8 h-5 w-5 text-ink-muted" />

      <div v-else-if="editorLoadError" class="flex flex-col items-start gap-3">
        <DbAlert>{{ editorLoadError }}</DbAlert>
        <DbButton size="sm" variant="secondary" @click="openEditor">
          Try again
        </DbButton>
      </div>

      <div
        v-else-if="editorAwaitingReauth"
        class="flex flex-col items-start gap-3"
        data-testid="env-editor-awaiting-reauth">
        <DbAlert tone="info">
          Confirm your password in the dialog to load the secrets for editing.
        </DbAlert>
        <DbButton size="sm" variant="secondary" @click="openEditor">
          Try again
        </DbButton>
      </div>

      <template v-else-if="editorContent !== null">
        <EnvFileEditor
          v-model="editorContent"
          :issues="editorIssues"
          :disabled="editorSaving" />

        <DbAlert v-if="editorError">
          {{ editorError }}
        </DbAlert>

        <!-- Dry-run confirmation before applying -->
        <div
          v-if="editorDiff"
          data-testid="env-editor-diff"
          class="flex flex-col gap-3 rounded-[var(--radius-card)] border border-line bg-panel p-4">
          <div class="flex items-center gap-2">
            <DbBadge tone="accent">dry-run ok</DbBadge>
            <span class="text-xs text-ink-muted">
              nothing stored yet — review the effect
            </span>
          </div>
          <div
            v-for="group in diffGroups"
            :key="group.label"
            class="rounded-md border border-line-soft bg-canvas px-3 py-2">
            <p
              class="mb-1 font-mono text-xs uppercase tracking-wide text-ink-faint">
              {{ group.label }} ({{ group.keys.length }})
            </p>
            <p class="font-mono text-xs text-ink">
              {{ group.keys.join(", ") || "—" }}
            </p>
          </div>
          <div class="flex items-center justify-end gap-2">
            <DbButton
              variant="ghost"
              :disabled="editorSaving"
              @click="backToEditing">
              Back to editing
            </DbButton>
            <DbButton
              variant="primary"
              :loading="editorSaving"
              data-testid="env-editor-apply"
              @click="applyDraft">
              Apply changes
            </DbButton>
          </div>
        </div>
      </template>
    </template>

    <!-- Create / edit secret -->
    <DbModal
      :open="form !== null"
      :title="form?.mode === 'edit' ? 'Update secret' : 'Add secret'"
      @close="closeForm">
      <form
        v-if="form"
        class="flex flex-col gap-4"
        novalidate
        @submit.prevent="submitForm">
        <DbInput
          v-if="form.mode === 'create'"
          v-model="form.key"
          label="Key"
          name="key"
          placeholder="DATABASE_URL"
          mono
          hint="Letters, numbers, '_', '-'; cannot start with a number." />
        <div v-else class="flex flex-col gap-1.5">
          <span class="text-xs font-medium text-ink-muted">Key</span>
          <DbCode>{{ form.secret.key }}</DbCode>
          <span class="text-xs text-ink-muted">
            Current version: v{{ form.secret.version }}. Saving creates a new
            version.
          </span>
        </div>
        <DbTextarea
          v-model="form.value"
          label="Value"
          name="value"
          :rows="5"
          placeholder="paste the secret value"
          hint="Sent over HTTPS and stored encrypted; it is never shown in listings." />
        <p v-if="form.error" class="text-xs text-crit">
          {{ form.error }}
        </p>
        <div class="flex items-center justify-end gap-2">
          <DbButton variant="ghost" :disabled="saving" @click="closeForm">
            Cancel
          </DbButton>
          <DbButton variant="primary" type="submit" :loading="saving">
            {{ form.mode === "edit" ? "Save new version" : "Save secret" }}
          </DbButton>
        </div>
      </form>
    </DbModal>

    <!-- Delete secret -->
    <DbConfirmDialog
      :open="deleteTarget !== null"
      title="Delete secret"
      :description="`Deleting '${deleteTarget?.key}' permanently removes the current value. This is recorded in the audit log.`"
      :confirm-word="deleteTarget?.key"
      confirm-label="Delete secret"
      :loading="deleteLoading"
      :error="deleteError"
      @confirm="confirmDelete"
      @close="deleteTarget = null" />

    <!-- Discard unsaved editor changes when switching views -->
    <DbConfirmDialog
      :open="discardTarget !== null"
      title="Discard unsaved changes?"
      description="Your edits in the .env editor are not saved yet. Switching views discards them."
      confirm-label="Discard changes"
      tone="danger"
      @confirm="confirmDiscardSwitch"
      @close="discardTarget = null" />

    <!-- Import / export -->
    <ImportSecretsDialog
      :open="showImport"
      :environment-id="environmentId"
      @close="showImport = false" />
    <ExportSecretsDialog
      :open="showExport"
      :environment-id="environmentId"
      :environment-name="environmentName"
      :project-name="projectName"
      @close="showExport = false" />
  </div>
</template>
