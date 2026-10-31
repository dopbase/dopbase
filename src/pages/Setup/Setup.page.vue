<script setup lang="ts">
import { ref } from "vue";
import { useSetupController } from "./Setup.controller";
import SetupSteps from "./components/SetupSteps.vue";
import { AuthLayout } from "~/layouts";
import {
  DbAlert,
  DbButton,
  DbCheckbox,
  DbCode,
  DbInput,
  DbSegment,
  DbSkeleton,
} from "~/components/ui";
import {
  CheckIcon,
  CopyIcon,
  DownloadIcon,
  EyeIcon,
  EyeOffIcon,
  KeyIcon,
  UploadIcon,
  XIcon,
} from "~/assets/icons";
import { formatBytes } from "~/utils/format";
import type { PasswordStrength } from "./recovery-kit";

const {
  mode,
  step,
  host,
  setupToken,
  email,
  password,
  confirmPassword,
  strength,
  strengthLabel,
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
  masterKey,
  keyLoading,
  keyError,
  keyRevealed,
  keyCopied,
  kitDownloaded,
  kitStored,
  shownKey,
  canContinue,
  loadRecoveryKey,
  copyKey,
  downloadKit,
  finish,
} = useSetupController();

const MODES = [
  { value: "setup", label: "New server" },
  { value: "restore", label: "Restore from backup" },
];
const strengthBars: Record<PasswordStrength, number> = {
  empty: 0,
  short: 1,
  weak: 1,
  fair: 2,
  strong: 4,
};
const strengthColor: Record<PasswordStrength, string> = {
  empty: "bg-line",
  short: "bg-crit",
  weak: "bg-crit",
  fair: "bg-warn",
  strong: "bg-ok",
};

const fileInputRef = ref<HTMLInputElement | null>(null);
const keyFileInputRef = ref<HTMLInputElement | null>(null);

function pickFile(event: Event, apply: (file: File | null) => void): void {
  const files = (event.target as HTMLInputElement).files;
  apply(files && files.length > 0 ? files[0]! : null);
}

function clearKeyFile(): void {
  onMasterKeyFileSelected(null);
  if (keyFileInputRef.value) keyFileInputRef.value.value = "";
}
</script>

<template>
  <AuthLayout>
    <!-- Step 2: recovery kit -->
    <div v-if="step === 'recovery'" data-testid="recovery-kit">
      <h1 class="text-[28px] font-semibold leading-tight text-ink-strong">
        Save your recovery kit
      </h1>
      <p class="mt-3 text-sm leading-relaxed text-ink-muted">
        Every secret is encrypted with this server's master key, and so are its
        backups. To restore a backup on a new machine you need this key. Keep it
        somewhere that isn't this server.
      </p>

      <div class="mt-6 rounded-card border border-line bg-panel px-4 py-3">
        <div class="flex items-center justify-between gap-2">
          <p class="text-[13px] text-ink-muted">Master key</p>
          <div v-if="masterKey" class="flex items-center gap-1">
            <button
              type="button"
              class="cursor-pointer rounded-control p-1 text-ink-muted transition-colors hover:bg-line hover:text-ink-strong"
              :aria-label="keyRevealed ? 'Hide master key' : 'Show master key'"
              :aria-pressed="keyRevealed"
              @click="keyRevealed = !keyRevealed">
              <EyeOffIcon v-if="keyRevealed" class="h-4 w-4" />
              <EyeIcon v-else class="h-4 w-4" />
            </button>
            <button
              type="button"
              class="cursor-pointer rounded-control p-1 text-ink-muted transition-colors hover:bg-line hover:text-ink-strong"
              :aria-label="keyCopied ? 'Master key copied' : 'Copy master key'"
              @click="copyKey">
              <CheckIcon v-if="keyCopied" class="h-4 w-4 text-ok" />
              <CopyIcon v-else class="h-4 w-4" />
            </button>
          </div>
        </div>
        <DbSkeleton v-if="keyLoading" class="mt-2 h-5 w-full rounded-control" />
        <p
          v-else-if="masterKey"
          class="mt-2 break-all font-mono text-[13px] text-ink-strong"
          data-testid="master-key">
          {{ shownKey }}
        </p>
        <div v-else-if="keyError" class="mt-2 flex flex-col items-start gap-2">
          <p class="text-sm text-crit">{{ keyError }}</p>
          <DbButton size="sm" @click="loadRecoveryKey">Try again</DbButton>
        </div>
      </div>

      <DbButton
        variant="primary"
        class="mt-3 w-full"
        :disabled="!masterKey"
        data-testid="download-kit"
        @click="downloadKit">
        <CheckIcon v-if="kitDownloaded" class="h-4 w-4" />
        <DownloadIcon v-else class="h-4 w-4" />
        {{
          kitDownloaded ? "Downloaded. Download again" : "Download recovery kit"
        }}
      </DbButton>
      <p class="mt-2 text-[13px] leading-relaxed text-ink-muted">
        A text file with the master key, this server's address and how to
        restore. Print it or put it in your password manager.
      </p>

      <div class="mt-6">
        <DbCheckbox
          v-model="kitStored"
          label="I've stored the recovery kit outside this server"
          :disabled="!masterKey" />
      </div>
      <DbButton
        class="mt-3 w-full"
        :variant="canContinue ? 'primary' : 'secondary'"
        :disabled="!canContinue"
        data-testid="setup-continue"
        @click="finish">
        Continue
      </DbButton>
      <p class="mt-2 text-[13px] text-ink-muted">
        Root can download it again later from Status.
        <button
          v-if="keyError"
          type="button"
          class="cursor-pointer underline underline-offset-4 hover:text-ink"
          @click="finish">
          Skip for now
        </button>
      </p>
    </div>

    <!-- Step 1: root account, or restore -->
    <div v-else>
      <h1 class="text-[28px] font-semibold leading-tight text-ink-strong">
        {{ mode === "setup" ? "Create the root account" : "Restore a backup" }}
      </h1>
      <p class="mt-3 text-sm leading-relaxed text-ink-muted">
        {{
          mode === "setup"
            ? "Root is for setup and recovery. Day to day, everyone, including you, signs in with their own account."
            : "Bring back every project, account and secret from a .dop backup. You sign in with an account from the backup afterwards."
        }}
      </p>

      <DbSegment
        :model-value="mode"
        :options="MODES"
        label="Setup mode"
        class="mt-6"
        @update:model-value="mode = $event as 'setup' | 'restore'" />

      <form
        v-if="mode === 'setup'"
        class="mt-5 flex flex-col gap-3"
        novalidate
        data-testid="setup-form"
        @submit.prevent="submit">
        <DbInput
          v-model="setupToken"
          label="Setup token"
          name="setupToken"
          placeholder="setup_..."
          mono
          hint="Printed by the server when it started. The setup link fills it in."
          :error="fieldErrors.setupToken" />
        <DbInput
          v-model="email"
          label="Email"
          name="email"
          type="email"
          autocomplete="email"
          placeholder="you@example.com"
          :error="fieldErrors.email" />
        <div class="flex flex-col gap-1.5">
          <DbInput
            v-model="password"
            label="Password"
            name="password"
            type="password"
            autocomplete="new-password"
            :error="fieldErrors.password" />
          <div
            v-if="strength !== 'empty'"
            class="flex flex-col gap-1"
            data-testid="password-strength">
            <div class="flex gap-1" aria-hidden="true">
              <span
                v-for="bar in 4"
                :key="bar"
                class="h-1 flex-1 rounded-control"
                :class="
                  bar <= strengthBars[strength]
                    ? strengthColor[strength]
                    : 'bg-line'
                " />
            </div>
            <p class="text-[13px] text-ink-muted">
              <span class="font-medium text-ink">{{ strengthLabel }}</span>
              · {{ password.length }} characters. A long passphrase from your
              password manager is best.
            </p>
          </div>
        </div>
        <DbInput
          v-model="confirmPassword"
          label="Confirm password"
          name="confirmPassword"
          type="password"
          autocomplete="new-password"
          :error="fieldErrors.confirmPassword" />

        <DbAlert v-if="formError">{{ formError }}</DbAlert>

        <DbButton
          variant="primary"
          type="submit"
          class="mt-1"
          :loading="submitting">
          Create root account
        </DbButton>
      </form>

      <form
        v-else
        class="mt-5 flex flex-col gap-3"
        novalidate
        data-testid="restore-form"
        @submit.prevent="submitRestore">
        <DbInput
          v-model="setupToken"
          label="Setup token"
          name="setupToken"
          placeholder="setup_..."
          mono
          hint="Printed by the server when it started." />

        <input
          ref="fileInputRef"
          type="file"
          accept=".dop"
          class="hidden"
          @change="pickFile($event, onFileSelected)" />
        <button
          type="button"
          class="flex cursor-pointer flex-col items-center justify-center rounded-card border border-dashed border-line px-4 py-5 text-center transition-colors hover:border-accent hover:bg-raised/30"
          @click="fileInputRef?.click()">
          <UploadIcon class="h-6 w-6 text-ink-muted" />
          <span class="mt-2 text-sm font-medium text-ink-strong">
            {{ selectedFile ? selectedFile.name : "Choose a .dop backup" }}
          </span>
          <span class="mt-0.5 font-mono text-xs text-ink-muted">
            {{
              selectedFile
                ? formatBytes(selectedFile.size)
                : "Encrypted backup archive"
            }}
          </span>
        </button>

        <div class="rounded-card border border-line bg-panel p-4">
          <p
            class="flex items-center gap-2 text-sm font-medium text-ink-strong">
            <KeyIcon class="h-4 w-4 text-ink-muted" />
            Master key of the old server
          </p>
          <p class="mt-1 text-[13px] leading-relaxed text-ink-muted">
            Only when the backup comes from another server. Use its
            <DbCode>master.key</DbCode> file or the 64-character key from its
            recovery kit.
          </p>
          <input
            ref="keyFileInputRef"
            type="file"
            class="hidden"
            @change="pickFile($event, onMasterKeyFileSelected)" />
          <div
            v-if="masterKeyFile"
            class="mt-3 flex items-center justify-between gap-2 rounded-control bg-raised px-3 py-2">
            <span class="truncate font-mono text-xs text-ink-strong">
              {{ masterKeyFile.name }}
            </span>
            <button
              type="button"
              class="cursor-pointer rounded-control p-1 text-ink-muted transition-colors hover:bg-line hover:text-crit"
              aria-label="Remove key file"
              @click="clearKeyFile">
              <XIcon class="h-3.5 w-3.5" />
            </button>
          </div>
          <div v-else class="mt-3 flex flex-col gap-2">
            <DbInput
              v-model="masterKeyHex"
              label="Master key (hex)"
              name="masterKeyHex"
              placeholder="4a2f8b…"
              mono />
            <DbButton
              size="sm"
              class="self-start"
              @click="keyFileInputRef?.click()">
              <UploadIcon class="h-3.5 w-3.5" />
              Use a master.key file
            </DbButton>
          </div>
        </div>

        <DbAlert v-if="restoreError">{{ restoreError }}</DbAlert>

        <DbButton
          variant="primary"
          type="submit"
          :disabled="!selectedFile"
          :loading="restoring">
          Restore backup
        </DbButton>
      </form>
    </div>

    <template #footer>
      <span>Self-hosted Dopbase</span>
      <span class="font-mono">{{ host }}</span>
    </template>

    <template #aside>
      <SetupSteps :host="host" :current="step === 'recovery' ? 2 : 1" />
    </template>
  </AuthLayout>
</template>
