<script setup lang="ts">
import { useAccountController } from "./Account.controller";
import { DashboardLayout } from "~/layouts";
import { DbAlert, DbBadge, DbButton, DbCode, DbInput } from "~/components/ui";
import { UserIcon } from "~/assets/icons";

/**
 * Account — the single admin's profile: email, session status, and
 * password rotation with session invalidation.
 */
const {
  email,
  recentAuthentication,
  currentPassword,
  newPassword,
  confirmPassword,
  fieldErrors,
  formError,
  submitting,
  submit,
} = useAccountController();
</script>

<template>
  <DashboardLayout>
    <div class="mx-auto max-w-xl p-8">
      <header class="mb-6">
        <h1 class="text-lg font-semibold">Account</h1>
        <p class="mt-0.5 text-sm text-ink-muted">
          v0.0.12 supports exactly one administrator.
        </p>
      </header>

      <section
        class="mb-4 rounded-[var(--radius-card)] border border-line bg-panel px-5 py-4">
        <div class="flex items-center gap-3">
          <div
            class="flex h-10 w-10 items-center justify-center rounded-lg border border-line bg-raised text-ink-muted">
            <UserIcon class="h-5 w-5" />
          </div>
          <div class="min-w-0">
            <p class="truncate font-mono text-sm text-ink-strong">
              {{ email }}
            </p>
            <p class="text-xs text-ink-muted">administrator</p>
          </div>
          <DbBadge
            class="ml-auto"
            :tone="recentAuthentication ? 'ok' : 'neutral'">
            {{ recentAuthentication ? "recently authenticated" : "session" }}
          </DbBadge>
        </div>
      </section>

      <section
        class="rounded-[var(--radius-card)] border border-line bg-panel px-5 py-4">
        <h2 class="text-sm font-semibold">Change password</h2>
        <p class="mt-0.5 text-xs text-ink-muted">
          Rotating the password signs out every session, including this one.
        </p>

        <form
          class="mt-4 flex flex-col gap-4"
          novalidate
          data-testid="change-password-form"
          @submit.prevent="submit">
          <DbInput
            v-model="currentPassword"
            label="Current password"
            name="currentPassword"
            type="password"
            autocomplete="current-password"
            :error="fieldErrors.currentPassword" />
          <DbInput
            v-model="newPassword"
            label="New password"
            name="newPassword"
            type="password"
            autocomplete="new-password"
            hint="12–128 characters."
            :error="fieldErrors.newPassword" />
          <DbInput
            v-model="confirmPassword"
            label="Confirm new password"
            name="confirmPassword"
            type="password"
            autocomplete="new-password"
            :error="fieldErrors.confirmPassword" />

          <DbAlert v-if="formError">{{ formError }}</DbAlert>

          <div>
            <DbButton variant="primary" type="submit" :loading="submitting">
              Change password
            </DbButton>
          </div>
        </form>
      </section>

      <p class="mt-4 text-sm leading-relaxed text-ink-muted">
        Forgot the password instead? Recovery is offline:
        <DbCode>dopbase admin reset-password</DbCode> on the server host with
        the master key.
      </p>
    </div>
  </DashboardLayout>
</template>
