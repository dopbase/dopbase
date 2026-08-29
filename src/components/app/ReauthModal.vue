<script setup lang="ts">
import { useReauthentication } from "~/composable";
import { LockIcon } from "~/assets/icons";
import { DbModal, DbInput, DbButton, DbAlert } from "~/components/ui";
import { ref } from "vue";

/**
 * ReauthModal — global "confirm your password" dialog.
 *
 * Mounted once inside `DashboardLayout`; opens automatically whenever any
 * API call answers 403 RECENT_AUTHENTICATION_REQUIRED (reveal, export) and
 * re-runs the parked action after a successful confirmation.
 */
const { isOpen, error, submitting, submit, dismiss } = useReauthentication();

const password = ref("");

async function onSubmit(): Promise<void> {
  const ok = await submit(password.value);
  if (ok) password.value = "";
}
</script>

<template>
  <DbModal
    :open="isOpen"
    title="Confirm your password"
    size="sm"
    persistent
    @close="dismiss">
    <form class="flex flex-col gap-4" novalidate @submit.prevent="onSubmit">
      <div class="flex items-start gap-3">
        <div
          class="flex h-8 w-8 shrink-0 items-center justify-center rounded-lg border border-warn/30 bg-warn/10 text-warn">
          <LockIcon class="h-4 w-4" />
        </div>
        <p class="text-sm text-ink">
          This action exposes secret material and needs a recent password
          confirmation.
        </p>
      </div>

      <DbInput
        v-model="password"
        label="Password"
        type="password"
        autocomplete="current-password"
        name="password"
        required
        autofocus />

      <DbAlert v-if="error">{{ error }}</DbAlert>

      <div class="flex items-center justify-end gap-2">
        <DbButton variant="ghost" :disabled="submitting" @click="dismiss">
          Cancel
        </DbButton>
        <DbButton
          variant="primary"
          type="submit"
          :loading="submitting"
          :disabled="password.length === 0">
          Confirm
        </DbButton>
      </div>
    </form>
  </DbModal>
</template>
