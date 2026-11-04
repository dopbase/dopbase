<script setup lang="ts">
import { computed, onMounted } from "vue";
import { useRoute } from "vue-router";
import { FocusLayout } from "~/layouts";
import { DbAlert } from "~/components/ui";
import { CheckIcon } from "~/assets/icons";
import { ssoErrorMessage } from "~/utils/sso";

/**
 * SsoConfirmed — where the provider sends the confirmation popup back.
 * The page that opened the popup notices the confirmation on its own; this
 * one only reports the result and closes itself.
 */
const route = useRoute();
const error = computed(() => ssoErrorMessage(route.query.sso_error));

onMounted(() => {
  // Browsers only let a script close a window that a script opened.
  if (!error.value && window.opener) {
    setTimeout(() => window.close(), 600);
  }
});
</script>

<template>
  <FocusLayout>
    <div v-if="error" class="flex flex-col gap-5">
      <h1 class="text-2xl font-semibold text-ink-strong">Not confirmed</h1>
      <DbAlert>{{ error }}</DbAlert>
      <p class="text-sm text-ink-muted">
        Close this window and try again from Dopbase.
      </p>
    </div>
    <div v-else class="flex flex-col items-center gap-5 text-center">
      <div
        class="flex h-14 w-14 items-center justify-center rounded-control bg-ok/15 text-ok">
        <CheckIcon class="h-7 w-7" />
      </div>
      <div class="flex flex-col gap-2">
        <h1 class="text-2xl font-semibold text-ink-strong">Confirmed</h1>
        <p class="text-sm text-ink-muted">
          This window closes on its own. If it doesn't, close it and go back to
          Dopbase.
        </p>
      </div>
    </div>
  </FocusLayout>
</template>
