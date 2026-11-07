<script setup lang="ts">
import { useLaunchController } from "./Launch.controller";
import { FocusLayout } from "~/layouts";
import { DbAlert, DbButton, DbSpinner } from "~/components/ui";

/** Launch — signs in with a passcode from `dopbase admin passcode`. */
const { error } = useLaunchController();
</script>

<template>
  <FocusLayout>
    <div v-if="error" class="flex flex-col gap-5">
      <h1 class="text-2xl font-semibold text-ink-strong">
        This launch link didn't work
      </h1>
      <DbAlert>{{ error }}</DbAlert>
      <RouterLink :to="{ name: 'login' }" class="w-full">
        <DbButton variant="primary" size="lg" class="w-full">
          Go to sign in
        </DbButton>
      </RouterLink>
    </div>
    <div
      v-else
      class="flex flex-col items-center gap-3 py-10 text-sm text-ink-muted"
      data-testid="launch-pending">
      <DbSpinner />
      Signing you in…
    </div>
  </FocusLayout>
</template>
