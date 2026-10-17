<script setup lang="ts">
import { computed, onMounted } from "vue";
import { useRoute } from "vue-router";
import { useLoginController } from "./Login.controller";
import { AuthLayout } from "~/layouts";
import { DbAlert, DbButton, DbCode, DbInput } from "~/components/ui";
import { TerminalIcon, InfoIcon } from "~/assets/icons";

const {
  email,
  password,
  fieldErrors,
  formError,
  submitting,
  ssoEnabled,
  submit,
  loadSsoConfig,
  ssoSignIn,
} = useLoginController();
const route = useRoute();
const notice = computed(() => {
  if (route.query.notice === "password-changed") {
    return "Your password was changed. Sign in with the new password.";
  }
  if (route.query.notice === "backup-restored") {
    return "Backup restored successfully. Sign in with an administrator account from the backup.";
  }
  if (route.query.notice === "oidc-failed") {
    return "Single sign-on failed. Try again, or sign in with your password.";
  }
  if (route.query.notice === "oidc-denied") {
    return "Your single sign-on account is not authorized for this instance.";
  }
  return null;
});

onMounted(loadSsoConfig);
</script>

<template>
  <AuthLayout>
    <div class="w-full max-w-sm">
      <p class="mb-1 font-mono text-xs text-ink-faint">$ dopbase login</p>
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

      <template v-if="ssoEnabled">
        <div class="my-5 flex items-center gap-3 text-xs text-ink-faint">
          <span class="h-px flex-1 bg-line-soft" />
          or
          <span class="h-px flex-1 bg-line-soft" />
        </div>
        <DbButton
          variant="secondary"
          type="button"
          data-testid="sso-sign-in"
          @click="ssoSignIn">
          Sign in with single sign-on
        </DbButton>
      </template>

      <details class="group mt-6 rounded-control border border-line bg-panel">
        <summary
          class="flex cursor-pointer list-none items-center gap-2 px-4 py-3 text-sm text-ink-muted transition-colors hover:text-ink">
          <InfoIcon class="h-4 w-4 shrink-0" aria-hidden="true" />
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
