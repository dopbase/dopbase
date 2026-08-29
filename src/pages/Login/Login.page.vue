<script setup lang="ts">
import { computed } from "vue";
import { useRoute } from "vue-router";
import { useLoginController } from "./Login.controller";
import { AuthLayout } from "~/layouts";
import { DbAlert, DbButton, DbCode, DbInput } from "~/components/ui";
import { TerminalIcon } from "~/assets/icons";

const { email, password, fieldErrors, formError, submitting, submit } =
  useLoginController();
const route = useRoute();
const notice = computed(() =>
  route.query.notice === "password-changed"
    ? "Your password was changed. Sign in with the new password."
    : null,
);
</script>

<template>
  <AuthLayout>
    <div class="w-full max-w-sm">
      <p class="mb-1 font-mono text-xs text-ink-faint">$ dopbase auth login</p>
      <h1 class="text-xl font-semibold">Sign in</h1>
      <p class="mt-1 text-sm text-ink-muted">
        Authenticate with the administrator account created during setup.
      </p>

      <form
        class="mt-6 flex flex-col gap-4"
        novalidate
        data-testid="login-form"
        @submit.prevent="submit">
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
          autocomplete="current-password"
          :error="fieldErrors.password" />

        <DbAlert v-if="notice" tone="info">{{ notice }}</DbAlert>
        <DbAlert v-if="formError">{{ formError }}</DbAlert>

        <DbButton variant="primary" type="submit" :loading="submitting">
          Sign in
        </DbButton>
      </form>

      <details class="group mt-6 rounded-md border border-line bg-panel">
        <summary
          class="cursor-pointer list-none px-4 py-3 text-sm text-ink-muted transition-colors hover:text-ink">
          Lost access?
        </summary>
        <div
          class="border-t border-line-soft px-4 py-3 text-xs leading-relaxed text-ink-muted">
          <p>
            Password recovery is an offline operation performed on the server
            host. Stop the server, then run:
          </p>
          <DbCode class="mt-2">dopbase admin reset-password</DbCode>
          <p class="mt-2">
            The command verifies the master key locally. The key is never
            entered in the browser or sent over HTTP.
          </p>
        </div>
      </details>

      <p class="mt-6 flex items-center gap-1.5 text-xs text-ink-muted">
        <TerminalIcon class="h-3.5 w-3.5" />
        Sessions expire after 8 hours of inactivity.
      </p>
    </div>
  </AuthLayout>
</template>
