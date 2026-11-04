<script setup lang="ts">
import { useLoginHelpController } from "./LoginHelp.controller";
import { AuthLayout } from "~/layouts";
import { DbCode } from "~/components/ui";
import { ArrowLeftIcon } from "~/assets/icons";

const { host, ssoName } = useLoginHelpController();
</script>

<template>
  <AuthLayout>
    <h1 class="text-[28px] font-semibold leading-tight text-ink-strong">
      Reset your password
    </h1>
    <p class="mt-3 text-sm leading-relaxed text-ink">
      Ask an admin of <span class="font-mono">{{ host }}</span> to set a new
      password for you from Users. Dopbase doesn't send reset emails.
    </p>
    <p v-if="ssoName" class="mt-3 text-sm leading-relaxed text-ink-muted">
      If you sign in with {{ ssoName }}, reset your password there instead.
    </p>

    <div class="mt-6 rounded-card border border-line bg-panel px-4 py-3">
      <p class="text-[13px] text-ink-muted">
        Root, on the server, with Dopbase stopped
      </p>
      <DbCode class="mt-2"
        >dopbase admin reset-password root@example.com</DbCode
      >
      <p class="mt-2 text-[13px] text-ink-muted">
        It checks the master key on the server. The key never goes through the
        browser.
      </p>
    </div>

    <RouterLink
      :to="{ name: 'login' }"
      class="mt-8 inline-flex h-9 items-center gap-2 rounded-control bg-raised px-3.5 text-sm font-medium text-ink-strong transition-colors hover:bg-line">
      <ArrowLeftIcon class="h-4 w-4" />
      Back to sign in
    </RouterLink>
  </AuthLayout>
</template>
