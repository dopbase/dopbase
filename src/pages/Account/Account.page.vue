<script setup lang="ts">
import { useAccountController } from "./Account.controller";
import { DashboardLayout } from "~/layouts";
import {
  DbAlert,
  DbBadge,
  DbButton,
  DbCode,
  DbCopyButton,
  DbInput,
} from "~/components/ui";
import { TerminalIcon, UserIcon } from "~/assets/icons";

/**
 * Account — profile overview, active session security status,
 * password rotation, and offline emergency recovery.
 */
const {
  email,
  formattedRole,
  lastLoginAt,
  formattedLastLogin,
  fullLastLogin,
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
    <div class="mx-auto max-w-4xl p-8">
      <!-- Header -->
      <header class="mb-6 flex items-center justify-between">
        <div>
          <div class="flex items-center gap-2.5">
            <h1 class="text-lg font-semibold text-ink-strong">Account</h1>
            <DbBadge :tone="recentAuthentication ? 'ok' : 'neutral'">
              {{
                recentAuthentication
                  ? "Recently authenticated"
                  : "Active session"
              }}
            </DbBadge>
          </div>
          <p class="mt-1 text-sm text-ink-muted">
            Manage your personal profile, credentials, and active session.
          </p>
        </div>
      </header>

      <div class="grid grid-cols-1 items-start gap-6 lg:grid-cols-3">
        <!-- Left column: Profile & Offline Recovery -->
        <div class="space-y-6 lg:col-span-1">
          <!-- Profile Card -->
          <section
            class="rounded-card border border-line bg-panel p-5 shadow-xs"
            aria-labelledby="profile-heading">
            <div class="flex items-center gap-3.5">
              <div
                class="flex h-11 w-11 shrink-0 items-center justify-center rounded-control border border-line bg-raised text-ink-muted">
                <UserIcon class="h-6 w-6" />
              </div>
              <div class="min-w-0 flex-1">
                <p
                  class="truncate font-mono text-xs font-semibold text-ink-strong"
                  :title="email">
                  {{ email }}
                </p>
                <p class="mt-0.5 text-xs text-ink-muted">
                  {{ formattedRole }}
                </p>
              </div>
            </div>

            <div
              class="mt-4 space-y-3 border-t border-line-soft pt-4 mb-2 text-xs">
              <div class="flex items-center justify-between gap-2">
                <span class="text-ink-muted">Last sign in</span>
                <span
                  v-if="lastLoginAt"
                  class="font-mono text-ink"
                  :title="fullLastLogin">
                  {{ formattedLastLogin }}
                </span>
                <span v-else class="text-ink-faint">—</span>
              </div>
            </div>
          </section>
        </div>

        <!-- Right column: Password Rotation -->
        <div class="space-y-6 lg:col-span-2">
          <!-- Offline Recovery Card -->
          <section
            class="rounded-card border border-line bg-panel p-5 shadow-xs"
            aria-labelledby="recovery-heading">
            <div class="flex items-center gap-2">
              <TerminalIcon class="h-4 w-4 text-ink-muted" />
              <h2
                id="recovery-heading"
                class="text-sm font-semibold text-ink-strong">
                Offline Recovery
              </h2>
            </div>
            <p class="mt-2 text-xs leading-relaxed text-ink-muted">
              Lost access? Reset credentials directly on the host server using
              the CLI:
            </p>

            <div
              class="mt-3 flex items-center justify-between gap-2 rounded-control border border-line bg-raised/70 px-3 py-2">
              <DbCode class="truncate text-xs"
                >dopbase admin reset-password</DbCode
              >
              <DbCopyButton value="dopbase admin reset-password" />
            </div>
          </section>

          <section
            class="rounded-card border border-line bg-panel p-6 shadow-xs"
            aria-labelledby="password-heading">
            <div class="border-b border-line-soft pb-4">
              <h2
                id="password-heading"
                class="text-sm font-semibold text-ink-strong">
                Change password
              </h2>
              <p class="mt-1 text-xs text-ink-muted">
                Rotating your password immediately invalidates all active
                sessions across devices, including this one.
              </p>
            </div>

            <form
              class="mt-5 flex flex-col gap-4"
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

              <div class="grid grid-cols-1 gap-4 sm:grid-cols-2">
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
              </div>

              <DbAlert v-if="formError" tone="error">
                {{ formError }}
              </DbAlert>

              <div
                class="mt-2 flex items-center justify-end border-t border-line-soft pt-4">
                <DbButton variant="primary" type="submit" :loading="submitting">
                  Change password
                </DbButton>
              </div>
            </form>
          </section>
        </div>
      </div>
    </div>
  </DashboardLayout>
</template>
