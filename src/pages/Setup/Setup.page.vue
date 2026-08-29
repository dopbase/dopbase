<script setup lang="ts">
import { useSetupController } from "./Setup.controller";
import { AuthLayout } from "~/layouts";
import { DbAlert, DbButton, DbCode, DbInput } from "~/components/ui";
import { KeyIcon, ShieldIcon } from "~/assets/icons";

const {
  setupToken,
  email,
  password,
  confirmPassword,
  fieldErrors,
  formError,
  submitting,
  submit,
} = useSetupController();
</script>

<template>
  <AuthLayout>
    <div class="w-full max-w-md">
      <p class="mb-1 font-mono text-xs text-ink-faint">
        $ dopbase setup --claim
      </p>
      <h1 class="text-xl font-semibold">Set up your server</h1>
      <p class="mt-1 text-sm text-ink-muted">
        This server is uninitialized. Claim it with the one-time setup token
        printed in the terminal when it started.
      </p>

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
        Only one administrator account exists in v0.0.1. Password recovery later
        requires the master key on the server host via
        <DbCode>dopbase admin reset-password</DbCode>.
      </p>
    </div>
  </AuthLayout>
</template>
