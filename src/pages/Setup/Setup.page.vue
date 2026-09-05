<script setup lang="ts">
import { ref } from "vue";
import { useSetupController } from "./Setup.controller";
import { AuthLayout } from "~/layouts";
import { DbAlert, DbButton, DbCode, DbInput } from "~/components/ui";
import { ArchiveIcon, KeyIcon, ShieldIcon, UploadIcon } from "~/assets/icons";
import { formatBytes } from "~/utils/format";

const {
  mode,
  setupToken,
  email,
  password,
  confirmPassword,
  fieldErrors,
  formError,
  submitting,
  submit,
  selectedFile,
  masterKeyFile,
  masterKeyHex,
  restoring,
  restoreError,
  onFileSelected,
  onMasterKeyFileSelected,
  submitRestore,
} = useSetupController();

const fileInputRef = ref<HTMLInputElement | null>(null);
const keyFileInputRef = ref<HTMLInputElement | null>(null);

function chooseFile() {
  fileInputRef.value?.click();
}

function handleFileInputChange(event: Event) {
  const target = event.target as HTMLInputElement;
  if (target.files && target.files.length > 0) {
    onFileSelected(target.files[0]);
  }
}

function chooseKeyFile() {
  keyFileInputRef.value?.click();
}

function handleKeyFileInputChange(event: Event) {
  const target = event.target as HTMLInputElement;
  if (target.files && target.files.length > 0) {
    onMasterKeyFileSelected(target.files[0]);
  }
}
</script>

<template>
  <AuthLayout>
    <div class="w-full max-w-md">
      <p class="mb-1 font-mono text-xs text-ink-faint">$ dopbase setup</p>
      <h1 class="text-xl font-semibold">Set up your server</h1>
      <p class="mt-1 text-sm text-ink-muted">
        This server is uninitialized. Claim it with your setup token, or enter
        that same token when restoring an existing backup archive.
      </p>

      <!-- Mode switcher tabs -->
      <div class="mt-4 flex rounded-md border border-line bg-raised/40 p-1">
        <button
          type="button"
          class="flex-1 rounded px-3 py-1.5 text-xs font-medium transition-colors"
          :class="
            mode === 'setup'
              ? 'bg-panel text-ink-strong shadow-sm'
              : 'text-ink-muted hover:text-ink'
          "
          data-testid="mode-setup-btn"
          @click="mode = 'setup'">
          New Instance
        </button>
        <button
          type="button"
          class="flex-1 rounded px-3 py-1.5 text-xs font-medium transition-colors"
          :class="
            mode === 'restore'
              ? 'bg-panel text-ink-strong shadow-sm'
              : 'text-ink-muted hover:text-ink'
          "
          data-testid="mode-restore-btn"
          @click="mode = 'restore'">
          Restore from Backup
        </button>
      </div>

      <!-- Mode 1: Claim new instance -->
      <template v-if="mode === 'setup'">
        <form
          class="mt-6 flex flex-col gap-5"
          novalidate
          data-testid="setup-form"
          @submit.prevent="submit">
          <!-- Claim instance -->
          <fieldset
            class="flex flex-col gap-4 rounded-[var(--radius-card)] border border-line bg-panel p-4">
            <legend
              class="flex items-center gap-1.5 px-1 font-mono text-xs text-accent-strong">
              <KeyIcon class="h-3.5 w-3.5" />
              claim-instance
            </legend>
            <DbInput
              v-model="setupToken"
              label="Setup token"
              name="setupToken"
              placeholder="dbs_..."
              mono
              hint="Shown once at server startup; it never appears in the UI."
              :error="fieldErrors.setupToken" />
          </fieldset>

          <!-- Admin account -->
          <fieldset
            class="flex flex-col gap-4 rounded-[var(--radius-card)] border border-line bg-panel p-4">
            <legend
              class="flex items-center gap-1.5 px-1 font-mono text-xs text-accent-strong">
              <ShieldIcon class="h-3.5 w-3.5" />
              admin-account
            </legend>
            <DbInput
              v-model="email"
              label="Email"
              name="email"
              type="email"
              autocomplete="email"
              placeholder="admin@example.com"
              :error="fieldErrors.email" />
            <DbInput
              v-model="password"
              label="Password"
              name="password"
              type="password"
              autocomplete="new-password"
              hint="12–128 characters."
              :error="fieldErrors.password" />
            <DbInput
              v-model="confirmPassword"
              label="Confirm password"
              name="confirmPassword"
              type="password"
              autocomplete="new-password"
              :error="fieldErrors.confirmPassword" />
          </fieldset>

          <DbAlert v-if="formError">{{ formError }}</DbAlert>

          <DbButton variant="primary" type="submit" :loading="submitting">
            Create admin &amp; sign in
          </DbButton>
        </form>

        <p class="mt-6 text-sm leading-relaxed text-ink-muted">
          Only one administrator account exists in v0.0.14. Password recovery
          later requires the master key on the server host via
          <DbCode>dopbase admin reset-password</DbCode>.
        </p>
      </template>

      <!-- Mode 2: Restore from backup archive -->
      <template v-else>
        <form
          class="mt-6 flex flex-col gap-5"
          novalidate
          data-testid="restore-form"
          @submit.prevent="submitRestore">
          <DbInput
            v-model="setupToken"
            label="Setup token"
            name="setupToken"
            placeholder="dbs_..."
            mono
            hint="Shown once at server startup; required to initialize this instance." />
          <fieldset
            class="flex flex-col gap-4 rounded-[var(--radius-card)] border border-line bg-panel p-4">
            <legend
              class="flex items-center gap-1.5 px-1 font-mono text-xs text-accent-strong">
              <ArchiveIcon class="h-3.5 w-3.5" />
              restore-backup
            </legend>

            <input
              ref="fileInputRef"
              type="file"
              accept=".dop"
              class="hidden"
              @change="handleFileInputChange" />

            <div
              class="flex cursor-pointer flex-col items-center justify-center rounded-lg border border-dashed border-line p-6 text-center transition-colors hover:border-accent hover:bg-raised/20"
              @click="chooseFile">
              <UploadIcon class="h-8 w-8 text-ink-muted" />
              <p class="mt-2 text-sm font-medium text-ink-strong">
                {{
                  selectedFile
                    ? selectedFile.name
                    : "Click to select a .dop backup file"
                }}
              </p>
              <p
                v-if="selectedFile"
                class="mt-1 font-mono text-xs text-ink-muted">
                {{ formatBytes(selectedFile.size) }}
              </p>
              <p v-else class="mt-1 text-xs text-ink-muted">
                Select an encrypted .dop backup archive.
              </p>
            </div>
          </fieldset>

          <!-- Master Key (optional, required if restoring from another server) -->
          <fieldset
            class="flex flex-col gap-3 rounded-[var(--radius-card)] border border-line bg-panel p-4">
            <legend
              class="flex items-center gap-1.5 px-1 font-mono text-xs text-accent-strong">
              <KeyIcon class="h-3.5 w-3.5" />
              master-key (required for new servers)
            </legend>
            <p class="text-xs text-ink-muted">
              Restoring onto a new server? Provide the original server's
              <DbCode>master.key</DbCode> or 64-character hex key. Dopbase will
              decrypt the snapshot and automatically re-key all secrets to this
              new server's master key.
            </p>

            <input
              ref="keyFileInputRef"
              type="file"
              class="hidden"
              @change="handleKeyFileInputChange" />

            <div class="flex items-center gap-2">
              <DbButton
                variant="secondary"
                size="sm"
                type="button"
                @click="chooseKeyFile">
                {{
                  masterKeyFile ? "Change key file" : "Select master.key file"
                }}
              </DbButton>
              <span
                v-if="masterKeyFile"
                class="font-mono text-xs text-ink-strong">
                {{ masterKeyFile.name }} ({{ formatBytes(masterKeyFile.size) }})
              </span>
              <span v-else class="text-xs text-ink-faint">
                Or paste 64-char hex key below
              </span>
            </div>

            <DbInput
              v-if="!masterKeyFile"
              v-model="masterKeyHex"
              label="Hex Master Key (optional)"
              name="masterKeyHex"
              placeholder="e.g. 4a2f8b..."
              mono />
          </fieldset>

          <DbAlert v-if="restoreError" tone="error">{{ restoreError }}</DbAlert>

          <DbButton
            variant="primary"
            type="submit"
            :disabled="!selectedFile"
            :loading="restoring">
            Restore &amp; initialize server
          </DbButton>
        </form>

        <p class="mt-6 text-sm leading-relaxed text-ink-muted">
          Restoring from a backup snapshot will restore all projects,
          environments, runner tokens, and the original administrator account.
          You will sign in with your snapshot credentials once restoration
          completes.
        </p>
      </template>
    </div>
  </AuthLayout>
</template>
