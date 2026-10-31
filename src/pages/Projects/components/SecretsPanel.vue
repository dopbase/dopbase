<script setup lang="ts">
import { computed, ref, toRef, watch } from "vue";
import { useRoute } from "vue-router";
import { useSecretsPanelController } from "./SecretsPanel.controller";
import { useImportSecretsDialogController } from "./ImportSecretsDialog.controller";
import EnvFileEditor from "./EnvFileEditor.vue";
import FirstRunChecklist from "./FirstRunChecklist.vue";
import ImportSecretsDialog from "./ImportSecretsDialog.vue";
import ExportSecretsDialog from "./ExportSecretsDialog.vue";
import SecretHistoryDialog from "./SecretHistoryDialog.vue";
import {
  DbAlert,
  DbBadge,
  DbButton,
  DbCheckbox,
  DbConfirmDialog,
  DbCopyButton,
  DbEmptyState,
  DbInput,
  DbMenu,
  DbMenuItem,
  DbModal,
  DbSearchField,
  DbSegment,
  DbSkeleton,
  DbTextarea,
} from "~/components/ui";
import {
  CopyIcon,
  DownloadIcon,
  EyeIcon,
  EyeOffIcon,
  HistoryIcon,
  KeyIcon,
  LockIcon,
  PencilIcon,
  PlusIcon,
  TrashIcon,
  UploadIcon,
} from "~/assets/icons";
import { formatRelativeTime } from "~/utils/format";
import { reaches } from "~/services/access.api";
import type { Level } from "~/services/access.api";
import { ApiError } from "~/services/http.client";
import { isValidSecretKey } from "~/utils/validation";
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
  /** Every environment in the project, for the first-run checklist. */
  environmentNames: string[];
  /** Your level here. Without one, every action shows. */
  level?: Level;
}>();

// The server checks every action; this only hides what would be refused.
const canRead = computed(() => !props.level || reaches(props.level, "read"));
const canWrite = computed(() => !props.level || reaches(props.level, "write"));

const environmentIdRef = computed(() => props.environmentId);
const controller = useSecretsPanelController(environmentIdRef);
// Files dropped on the first-run checklist go through the same parsing and
// review page as the Import dialog.
const dropImport = useImportSecretsDialogController(
  environmentIdRef,
  toRef(props, "environmentName"),
  ref(false),
);
const route = useRoute();
// Returning from the full-page import sets an `imported` timestamp in the
// query. Refetch so the table reflects what was just applied.
watch(
  () => route.query.imported,
  () => {
    if (route.name === "environment") void controller.reload();
  },
);
// Destructure so refs auto-unwrap in the template.
const {
  secrets,
  filter,
  visibleSecrets,
  copiedKey,
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
  | {
      mode: "create";
      key: string;
      value: string;
      useEmptyValue: boolean;
      valueDraft: string;
      error: string | null;
    }
  | {
      mode: "edit";
      secret: SecretMetadata;
      value: string;
      error: string | null;
    }
  | null;

type PanelView = "table" | "editor";

/** The key whose version history is open, if any. */
const historyKey = ref<string | null>(null);

function openHistory(key: string): void {
  form.value = null;
  historyKey.value = key;
}

function onRolledBack(): void {
  hideRevealed();
  void controller.reload();
}

const view = ref<PanelView>("table");

const viewOptions = computed(() =>
  canWrite.value
    ? [
        { value: "table", label: "Table" },
        { value: "editor", label: ".env" },
      ]
    : [{ value: "table", label: "Table" }],
);

/** An empty environment walks writers through their first secrets. */
const showChecklist = computed(
  () =>
    view.value === "table" &&
    canWrite.value &&
    secrets.value !== null &&
    secrets.value.length === 0 &&
    !loading.value,
);

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

/* Editor-diff styling: each group reads like a diff hunk. */
const diffGroupClasses: Record<string, string> = {
  Added: "border-ok/30 bg-ok/10",
  Updated: "border-warn/30 bg-warn/10",
  Deleted: "border-crit/30 bg-crit/10",
  Unchanged: "border-line-soft bg-canvas",
};
const diffGroupLabelClasses: Record<string, string> = {
  Added: "text-ok",
  Updated: "text-warn",
  Deleted: "text-crit",
  Unchanged: "text-ink-faint",
};

const form = ref<FormState>(null);
const saving = ref(false);
const deleteTarget = ref<SecretMetadata | null>(null);
const deleteLoading = ref(false);
const deleteError = ref<string | null>(null);
const showImport = ref(false);
const showExport = ref(false);

function openCreate(): void {
  form.value = {
    mode: "create",
    key: "",
    value: "",
    useEmptyValue: false,
    valueDraft: "",
    error: null,
  };
}

function openEdit(secret: SecretMetadata): void {
  hideRevealed();
  form.value = { mode: "edit", secret, value: "", error: null };
}

function closeForm(): void {
  if (!saving.value) form.value = null;
}

function setUseEmptyValue(enabled: boolean): void {
  const current = form.value;
  if (!current || current.mode !== "create") return;
  if (enabled) {
    current.valueDraft = current.value;
    current.value = "";
  } else {
    current.value = current.valueDraft;
    current.valueDraft = "";
  }
  current.useEmptyValue = enabled;
  current.error = null;
}

async function submitForm(): Promise<void> {
  if (!form.value) return;
  if (
    form.value.mode === "create" &&
    !isValidSecretKey(form.value.key.trim())
  ) {
    form.value.error =
      "Use 1 to 128 characters. Use letters, numbers, and '_', and start with a letter or '_'.";
    return;
  }
  if (
    form.value.mode === "create" &&
    !form.value.useEmptyValue &&
    form.value.value.length === 0
  ) {
    form.value.error = "Enter a value or select Use empty value.";
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
          ? "Keys may use letters, numbers, and '_', and cannot start with a number."
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
    <div
      v-if="!showChecklist"
      class="flex flex-wrap items-center gap-2"
      data-testid="secrets-toolbar">
      <DbSearchField
        v-if="view === 'table'"
        v-model="filter"
        class="w-full sm:w-[220px]"
        placeholder="Filter keys" />
      <div class="ml-auto flex flex-wrap items-center gap-2">
        <DbSegment
          :model-value="view"
          :options="viewOptions"
          label="Show secrets as"
          @update:model-value="switchView($event as PanelView)" />
        <template v-if="view === 'table'">
          <DbButton
            v-if="canRead"
            variant="ghost"
            data-testid="secrets-export"
            @click="showExport = true">
            <DownloadIcon class="h-4 w-4" />
            Export
          </DbButton>
          <template v-if="canWrite">
            <DbButton data-testid="secrets-import" @click="showImport = true">
              <UploadIcon class="h-4 w-4" />
              Import
            </DbButton>
            <DbButton
              variant="primary"
              data-testid="secrets-add"
              @click="openCreate">
              <PlusIcon class="h-4 w-4" />
              Add secret
            </DbButton>
          </template>
        </template>
      </div>
    </div>

    <!-- Table view -->
    <template v-if="view === 'table'">
      <DbAlert v-if="actionError">
        {{ actionError }}
      </DbAlert>

      <div
        v-if="loading"
        class="overflow-hidden rounded-card border border-line bg-panel"
        data-testid="secrets-skeleton">
        <div
          v-for="i in 5"
          :key="i"
          class="flex h-12 items-center gap-6 border-b border-line-soft px-4 last:border-b-0">
          <DbSkeleton class="h-4 w-40" />
          <DbSkeleton class="h-4 w-32" />
          <DbSkeleton class="ml-auto h-4 w-20" />
        </div>
      </div>

      <DbAlert v-else-if="loadError">
        {{ loadError }}
      </DbAlert>

      <FirstRunChecklist
        v-else-if="showChecklist"
        :project-name="projectName"
        :environment-name="environmentName"
        :environment-names="environmentNames"
        :importing="dropImport.parsing.value"
        @choose-file="showImport = true"
        @drop-file="dropImport.processFile"
        @paste-env="switchView('editor')"
        @add-one="openCreate" />

      <DbEmptyState
        v-else-if="secrets && secrets.length === 0"
        title="No secrets in this environment"
        description="Nobody has stored a secret here yet.">
        <template #icon>
          <KeyIcon class="h-5 w-5" />
        </template>
      </DbEmptyState>

      <div
        v-else-if="secrets && visibleSecrets"
        class="overflow-x-auto rounded-card border border-line bg-panel">
        <table class="min-w-full text-left text-sm" data-testid="secrets-table">
          <thead>
            <tr class="border-b border-line-soft">
              <th class="h-11 px-4 font-medium text-ink-muted">Key</th>
              <th
                class="hidden h-11 px-4 font-medium text-ink-muted sm:table-cell">
                Value
              </th>
              <th
                class="hidden h-11 px-4 font-medium text-ink-muted md:table-cell">
                Updated
              </th>
              <th class="h-11 w-12 px-2">
                <span class="sr-only">Actions</span>
              </th>
            </tr>
          </thead>
          <tbody>
            <tr v-if="visibleSecrets.length === 0">
              <td
                colspan="4"
                class="px-4 py-8 text-center text-ink-muted"
                data-testid="secrets-no-match">
                No keys match “{{ filter.trim() }}”.
              </td>
            </tr>
            <template v-for="secret in visibleSecrets" :key="secret.key">
              <tr
                class="group border-b border-line-soft transition-colors last:border-b-0 hover:bg-raised"
                data-testid="secret-row">
                <td class="h-12 w-full max-w-0 px-4 sm:w-[34%]">
                  <p
                    class="truncate font-mono text-sm leading-[22px] text-ink-strong"
                    :title="secret.key">
                    {{ secret.key }}
                  </p>
                  <p class="text-[13px] text-ink-muted md:hidden">
                    {{ formatRelativeTime(secret.updatedAt) }} · v{{
                      secret.version
                    }}
                  </p>
                </td>
                <td class="hidden h-12 px-4 sm:table-cell">
                  <span
                    v-if="copiedKey === secret.key"
                    class="font-mono text-[13px] text-ok"
                    role="status">
                    Copied
                  </span>
                  <button
                    v-else-if="canRead"
                    type="button"
                    class="cursor-pointer rounded-control px-1 font-mono text-[13px] leading-[22px] tracking-widest text-ink-muted transition-colors hover:text-ink-strong"
                    :aria-label="
                      revealedKey === secret.key
                        ? `Hide ${secret.key}`
                        : `Reveal ${secret.key}`
                    "
                    :title="revealedKey === secret.key ? 'Hide' : 'Reveal'"
                    @click="
                      revealedKey === secret.key
                        ? hideRevealed()
                        : reveal(secret.key)
                    ">
                    ••••••••••••••••
                  </button>
                  <span
                    v-else
                    class="text-[13px] text-ink-faint"
                    title="Your access shows names only">
                    Hidden
                  </span>
                </td>
                <td
                  class="hidden h-12 whitespace-nowrap px-4 text-[13px] text-ink-muted md:table-cell">
                  {{ formatRelativeTime(secret.updatedAt) }} · v{{
                    secret.version
                  }}
                </td>
                <td class="h-12 px-2 text-right">
                  <DbMenu :label="`Actions for ${secret.key}`">
                    <template v-if="canRead">
                      <DbMenuItem
                        v-if="revealedKey !== secret.key"
                        @select="reveal(secret.key)">
                        <template #icon><EyeIcon class="h-4 w-4" /></template>
                        Reveal
                      </DbMenuItem>
                      <DbMenuItem v-else @select="hideRevealed()">
                        <template #icon
                          ><EyeOffIcon class="h-4 w-4"
                        /></template>
                        Hide
                      </DbMenuItem>
                      <DbMenuItem @select="controller.copyValue(secret.key)">
                        <template #icon><CopyIcon class="h-4 w-4" /></template>
                        Copy value
                      </DbMenuItem>
                    </template>
                    <DbMenuItem @select="openHistory(secret.key)">
                      <template #icon><HistoryIcon class="h-4 w-4" /></template>
                      History
                    </DbMenuItem>
                    <template v-if="canWrite">
                      <DbMenuItem @select="openEdit(secret)">
                        <template #icon
                          ><PencilIcon class="h-4 w-4"
                        /></template>
                        Edit value
                      </DbMenuItem>
                      <div class="my-0.5 h-px bg-line-soft" role="separator" />
                      <DbMenuItem tone="danger" @select="deleteTarget = secret">
                        <template #icon><TrashIcon class="h-4 w-4" /></template>
                        Delete
                      </DbMenuItem>
                    </template>
                  </DbMenu>
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
      <!-- Slim toolbar: state badge + actions. Key/issue counts live in
           the editor's own status bar now. -->
      <div class="flex flex-wrap items-center gap-2">
        <DbBadge v-if="editorDirty" tone="warn">unsaved changes</DbBadge>
        <DbBadge v-else-if="editorContent !== null" tone="ok"> saved </DbBadge>
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

      <div
        v-if="editorLoading"
        class="space-y-3"
        data-testid="env-editor-skeleton">
        <DbSkeleton class="h-64 w-full rounded-card" />
      </div>

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
          :disabled="editorSaving"
          :dirty="editorDirty"
          :subtitle="environmentName"
          @save="saveDraft" />

        <DbAlert v-if="editorError">
          {{ editorError }}
        </DbAlert>

        <!-- Dry-run confirmation before applying -->
        <div
          v-if="editorDiff"
          data-testid="env-editor-diff"
          class="flex flex-col gap-3 rounded-card border border-line bg-panel p-4">
          <div class="flex flex-wrap items-center gap-2">
            <DbBadge tone="accent">Validate changes is ok</DbBadge>
            <span class="text-xs text-ink-muted">
              nothing stored yet, review the effect
            </span>
          </div>
          <div
            v-for="group in diffGroups"
            :key="group.label"
            class="rounded-control border px-3 py-2"
            :class="diffGroupClasses[group.label]">
            <p
              class="mb-1 font-mono text-xs font-semibold uppercase tracking-wide"
              :class="diffGroupLabelClasses[group.label]">
              {{ group.label }} ({{ group.keys.length }})
            </p>
            <p class="break-words font-mono text-xs text-ink">
              {{ group.keys.join(", ") || "—" }}
            </p>
          </div>
          <div class="flex flex-wrap items-center justify-end gap-2">
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
          hint="Use letters, numbers, '_', and '-'. The first character cannot be a number." />
        <div v-else class="flex flex-col gap-1.5">
          <div class="rounded-control bg-raised" data-testid="secret-key-field">
            <p class="px-3.5 pb-0.5 pt-2 text-xs font-semibold text-ink-muted">
              Key
            </p>
            <div class="flex h-9 items-center gap-2 px-3.5">
              <LockIcon
                class="h-3.5 w-3.5 shrink-0 text-ink-faint"
                aria-hidden="true" />
              <span
                class="min-w-0 flex-1 truncate font-mono text-xs text-ink-strong"
                :title="form.secret.key">
                {{ form.secret.key }}
              </span>
              <DbBadge>v{{ form.secret.version }}</DbBadge>
            </div>
          </div>
          <p class="text-xs text-ink-muted">
            Keys can't be renamed. Saving stores the new value as v{{
              form.secret.version + 1
            }}.
            <button
              type="button"
              class="cursor-pointer text-accent-strong underline-offset-2 hover:underline"
              @click="openHistory(form.secret.key)">
              See history
            </button>
          </p>
        </div>
        <DbTextarea
          v-model="form.value"
          label="Value"
          name="value"
          :rows="5"
          :disabled="form.mode === 'create' && form.useEmptyValue"
          :required="form.mode === 'create' && !form.useEmptyValue"
          placeholder="paste the secret value"
          :hint="
            form.mode === 'create' && form.useEmptyValue
              ? 'This secret will be saved with an empty value.'
              : 'Sent over HTTPS and stored encrypted, it is never shown in listings.'
          "
          @input="form.error = null" />
        <DbCheckbox
          v-if="form.mode === 'create'"
          :model-value="form.useEmptyValue"
          label="Use empty value"
          :disabled="saving"
          @update:model-value="setUseEmptyValue" />
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
      :description="`Deleting '${deleteTarget?.key}' permanently removes its value and its version history. This is recorded in the audit log.`"
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

    <!-- Version history and rollback -->
    <SecretHistoryDialog
      :environment-id="environmentId"
      :secret-key="historyKey"
      @close="historyKey = null"
      @rolled-back="onRolledBack" />

    <!-- Import / export -->
    <ImportSecretsDialog
      :open="showImport"
      :environment-id="environmentId"
      :environment-name="environmentName"
      @close="showImport = false" />
    <ExportSecretsDialog
      :open="showExport"
      :environment-name="environmentName"
      :project-name="projectName"
      :action="controller.exportSecretsFile"
      @close="showExport = false" />
  </div>
</template>
