<script setup lang="ts">
import { ref } from "vue";
import { useSetupController } from "./Setup.controller";
import { AuthLayout } from "~/layouts";
import { DbAlert, DbButton, DbCode, DbInput } from "~/components/ui";
import {
  ArchiveIcon,
  InfoIcon,
  KeyIcon,
  ShieldIcon,
  UploadIcon,
  XIcon,
} from "~/assets/icons";
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

function clearKeyFile() {
  onMasterKeyFileSelected(null);
  if (keyFileInputRef.value) {
    keyFileInputRef.value.value = "";
  }
}
</script>

<template>
  <AuthLayout>
    <div
      class="w-full max-w-md transition-[max-width] duration-200 lg:max-w-3xl">
      <p class="mb-1 font-mono text-xs text-ink-faint">$ dopbase setup</p>
      <h1 class="text-xl font-semibold">Set up your server</h1>
      <p class="mt-1 text-sm text-ink-muted">
        This server is uninitialized. Claim it with your setup token, or enter
        that same token when restoring an existing backup archive.
      </p>

      <!-- Mode switcher tabs -->
      <div
        class="mt-4 flex rounded-control border border-line bg-canvas p-1"
        role="tablist"
        aria-label="Setup mode">
        <button
          type="button"
          role="tab"
          :aria-selected="mode === 'setup'"
          class="flex-1 cursor-pointer rounded px-3 py-2 text-xs font-medium transition-all"
          :class="
            mode === 'setup'
              ? 'border border-accent/60 bg-raised font-semibold text-ink-strong shadow-sm'
              : 'border border-transparent text-ink-muted hover:bg-raised/30 hover:text-ink'
          "
          data-testid="mode-setup-btn"
          @click="mode = 'setup'">
          <span class="flex items-center justify-center gap-2">
            <span
              class="h-1.5 w-1.5 rounded-full transition-colors"
              :class="mode === 'setup' ? 'bg-accent' : 'bg-line-strong'" />
            New Instance
          </span>
        </button>
        <button
          type="button"
          role="tab"
          :aria-selected="mode === 'restore'"
          class="flex-1 cursor-pointer rounded px-3 py-2 text-xs font-medium transition-all"
          :class="
            mode === 'restore'
              ? 'border border-accent/60 bg-raised font-semibold text-ink-strong shadow-sm'
              : 'border border-transparent text-ink-muted hover:bg-raised/30 hover:text-ink'
          "
          data-testid="mode-restore-btn"
          @click="mode = 'restore'">
          <span class="flex items-center justify-center gap-2">
            <span
              class="h-1.5 w-1.5 rounded-full transition-colors"
              :class="mode === 'restore' ? 'bg-accent' : 'bg-line-strong'" />
            Restore from Backup
          </span>
        </button>
      </div>

      <!-- Mode 1: Claim new instance -->
      <template v-if="mode === 'setup'">
        <form
          class="mt-6 flex flex-col gap-4"
          novalidate
          data-testid="setup-form"
          @submit.prevent="submit">
          <div class="grid grid-cols-1 items-stretch gap-4 lg:grid-cols-2">
            <!-- Claim instance -->
            <fieldset
              class="flex h-full flex-col justify-between gap-4 rounded-card border border-line bg-panel p-4">
              <div class="flex flex-col gap-3">
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
                  hint="Printed once at startup, never appears in the UI."
                  :error="fieldErrors.setupToken" />
              </div>

              <!-- Information block balancing height with the 3 inputs on the right -->
              <div
                class="flex flex-col gap-2.5 rounded-control border border-line-soft bg-raised/30 p-3 text-xs leading-relaxed text-ink-muted">
                <div>
                  <p class="font-semibold text-ink-strong">
                    One-time bootstrap claim
                  </p>
                  <p class="mt-0.5">
                    The setup token is printed in your terminal or written to
                    <DbCode>~/.dopbase/serve.log</DbCode>. Opening the setup
                    link printed alongside it fills this field automatically. It
                    works once to claim ownership and initialize the instance.
                  </p>
                </div>
              </div>
            </fieldset>

            <!-- Admin account -->
            <fieldset
              class="flex h-full flex-col justify-between gap-3 rounded-card border border-line bg-panel p-4">
              <div>
                <legend
                  class="flex items-center gap-1.5 px-1 font-mono text-xs text-accent-strong">
                  <ShieldIcon class="h-3.5 w-3.5" />
                  create-root-account
                </legend>
                <div class="mt-1 flex flex-col gap-3">
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
                    placeholder="password"
                    :error="fieldErrors.password" />
                  <DbInput
                    v-model="confirmPassword"
                    label="Confirm password"
                    name="confirmPassword"
                    type="password"
                    autocomplete="new-password"
                    placeholder="password"
                    :error="fieldErrors.confirmPassword" />
                </div>
              </div>
            </fieldset>
          </div>

          <DbAlert v-if="formError">{{ formError }}</DbAlert>

          <DbButton variant="primary" type="submit" :loading="submitting">
            Create admin &amp; sign in
          </DbButton>

          <details class="group rounded-control border border-line bg-panel">
            <summary
              class="flex cursor-pointer list-none items-center gap-2 px-3.5 py-2.5 text-xs text-ink-muted transition-colors hover:text-ink">
              <InfoIcon class="h-3.5 w-3.5 shrink-0" aria-hidden="true" />
              Password recovery information
            </summary>
            <div
              class="border-t border-line-soft px-3.5 py-2.5 text-xs leading-relaxed text-ink-muted">
              <p>
                Only one root account exists in v0.0.14. Password recovery later
                requires the master key on the server host via:
              </p>
              <DbCode class="mt-2">dopbase admin reset-password</DbCode>
            </div>
          </details>
        </form>
      </template>

      <!-- Mode 2: Restore from backup archive -->
      <template v-else>
        <form
          class="mt-6 flex flex-col gap-4"
          novalidate
          data-testid="restore-form"
          @submit.prevent="submitRestore">
          <div class="grid grid-cols-1 items-stretch gap-4 lg:grid-cols-2">
            <!-- Restore backup -->
            <fieldset
              class="flex h-full flex-col justify-between gap-3 rounded-card border border-line bg-panel p-4">
              <div class="flex flex-col gap-3">
                <legend
                  class="flex items-center gap-1.5 px-1 font-mono text-xs text-accent-strong">
                  <ArchiveIcon class="h-3.5 w-3.5" />
                  restore-backup
                </legend>
                <DbInput
                  v-model="setupToken"
                  label="Setup token"
                  name="setupToken"
                  placeholder="dbs_..."
                  mono
                  hint="Shown once at server startup, required to initialize this instance." />
                <input
                  ref="fileInputRef"
                  type="file"
                  accept=".dop"
                  class="hidden"
                  @change="handleFileInputChange" />

                <div
                  class="flex cursor-pointer flex-col items-center justify-center rounded-lg border border-dashed border-line p-5 text-center transition-colors hover:border-accent hover:bg-raised/20"
                  @click="chooseFile">
                  <UploadIcon class="h-7 w-7 text-ink-muted" />
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
              </div>
            </fieldset>

            <!-- Master Key (optional, required if restoring from another server) -->
            <fieldset
              class="flex h-full flex-col justify-between gap-3 rounded-card border border-line bg-panel p-4">
              <div>
                <legend
                  class="flex items-center gap-1.5 px-1 font-mono text-xs text-accent-strong">
                  <KeyIcon class="h-3.5 w-3.5" />
                  master-key (for new servers)
                </legend>
                <p class="mt-1 text-xs leading-relaxed text-ink-muted">
                  Restoring onto a new server? Provide the original server's
                  <DbCode>master.key</DbCode> or 64-char hex key to decrypt and
                  automatically re-key all secrets to this server's master key.
                </p>
              </div>

              <input
                ref="keyFileInputRef"
                type="file"
                class="hidden"
                @change="handleKeyFileInputChange" />

              <div class="flex flex-col gap-2">
                <!-- If file selected -->
                <div
                  v-if="masterKeyFile"
                  class="flex min-w-0 items-center justify-between gap-2 rounded-control border border-line-soft bg-raised/40 p-2.5">
                  <div class="flex min-w-0 items-center gap-2">
                    <KeyIcon class="h-4 w-4 shrink-0 text-accent-strong" />
                    <div class="min-w-0">
                      <p
                        class="truncate font-mono text-xs font-medium text-ink-strong">
                        {{ masterKeyFile.name }}
                      </p>
                      <p class="font-mono text-[11px] text-ink-muted">
                        {{ formatBytes(masterKeyFile.size) }}
                      </p>
                    </div>
                  </div>
                  <button
                    type="button"
                    class="cursor-pointer shrink-0 rounded p-1 text-ink-muted transition-colors hover:bg-line hover:text-crit"
                    title="Remove key file"
                    aria-label="Remove key file"
                    @click="clearKeyFile">
                    <XIcon class="h-3.5 w-3.5" />
                  </button>
                </div>

                <!-- If no file selected -->
                <DbButton
                  v-else
                  variant="secondary"
                  size="sm"
                  type="button"
                  class="w-full justify-center"
                  @click="chooseKeyFile">
                  <UploadIcon class="h-3.5 w-3.5" />
                  Select master.key file
                </DbButton>

                <!-- OR divider -->
                <div class="relative my-0.5 flex items-center justify-center">
                  <div class="w-full border-t border-line-soft" />
                  <span
                    class="absolute bg-panel px-2 font-mono text-[11px] uppercase tracking-wider text-ink-faint">
                    or
                  </span>
                </div>

                <!-- Hex Master Key input -->
                <DbInput
                  v-model="masterKeyHex"
                  label="Hex Master Key (optional)"
                  name="masterKeyHex"
                  placeholder="e.g. 4a2f8b..."
                  mono
                  :disabled="!!masterKeyFile"
                  :hint="
                    masterKeyFile
                      ? 'Using uploaded master.key file. Click ✕ to clear.'
                      : 'Paste 64-char hex key from previous host.'
                  " />
              </div>
            </fieldset>
          </div>

          <DbAlert v-if="restoreError" tone="error">{{ restoreError }}</DbAlert>

          <DbButton
            variant="primary"
            type="submit"
            :disabled="!selectedFile"
            :loading="restoring">
            Restore &amp; initialize server
          </DbButton>

          <details class="group rounded-control border border-line bg-panel">
            <summary
              class="flex cursor-pointer list-none items-center gap-2 px-3.5 py-2.5 text-xs text-ink-muted transition-colors hover:text-ink">
              <InfoIcon class="h-3.5 w-3.5 shrink-0" aria-hidden="true" />
              Restore details &amp; next steps
            </summary>
            <div
              class="border-t border-line-soft px-3.5 py-2.5 text-xs leading-relaxed text-ink-muted">
              <p>
                Restoring from a backup snapshot will restore all projects,
                environments, runner tokens, and the original administrator
                account. You will sign in with your snapshot credentials once
                restoration completes.
              </p>
            </div>
          </details>
        </form>
      </template>
    </div>
  </AuthLayout>
</template>
