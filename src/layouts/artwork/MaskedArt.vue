<script setup lang="ts">
import art from "~/assets/artwork/masked.svg";
import { LockIcon } from "~/assets/icons";
import { artPx } from "./AuthArtwork.controller";

// Keys fade out from the highlighted one in the middle.
const keys = [
  { text: "API_KEY=••••••••••••••", top: 72, opacity: 0.07 },
  { text: "DATABASE_URL=•••••••••••••••••••••", top: 122, opacity: 0.15 },
  { text: "REDIS_URL=•••••••••••••••••••", top: 172, opacity: 0.23 },
  { text: "SENTRY_DSN=•••••••••••••••••", top: 222, opacity: 0.31 },
  { text: "JWT_SIGNING_KEY=•••••••••••••••", top: 272, opacity: 0.39 },
  { text: "OPENAI_API_KEY=••••••••••••••••••••••", top: 322, opacity: 0.47 },
  { text: "S3_ACCESS_KEY=••••••••••••••••••", top: 422, opacity: 0.47 },
  { text: "SMTP_PASSWORD=••••••••••••••••", top: 472, opacity: 0.39 },
  { text: "GITHUB_TOKEN=••••••••••••••", top: 522, opacity: 0.31 },
  { text: "SESSION_SECRET=•••••••••••••••••••••", top: 572, opacity: 0.23 },
  { text: "FEATURE_FLAGS=•••••••••••••••••••", top: 622, opacity: 0.15 },
  { text: "WEBHOOK_SECRET=•••••••••••••••••", top: 672, opacity: 0.07 },
];
</script>

<template>
  <div
    class="absolute inset-0 font-mono leading-[normal]"
    :style="{ fontSize: artPx(20) }"
    aria-hidden="true">
    <img :src="art" alt="" class="absolute inset-0 size-full" />
    <p
      v-for="key in keys"
      :key="key.text"
      class="absolute whitespace-nowrap text-ink"
      :style="{ left: artPx(80), top: artPx(key.top), opacity: key.opacity }">
      {{ key.text }}
    </p>
    <div
      class="absolute flex items-center border border-brand-mark/45 bg-brand-mark/12 text-brand-mark"
      :style="{
        left: artPx(64),
        top: artPx(362),
        gap: artPx(12),
        padding: `${artPx(10)} ${artPx(20)} ${artPx(10)} ${artPx(16)}`,
        borderRadius: artPx(10),
      }">
      <LockIcon
        class="shrink-0"
        :style="{ width: artPx(16), height: artPx(16) }" />
      <p class="whitespace-nowrap">PAYMENT_SECRET_KEY=••••••••••••••••••••</p>
    </div>
  </div>
</template>
