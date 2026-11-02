<script setup lang="ts">
import art from "~/assets/artwork/network.svg";
import { DopbaseIcon } from "~/assets/icons";
import { artPx } from "./AuthArtwork.controller";

const apps = [
  { name: "payments-api", left: 435, top: 132 },
  { name: "worker", left: 658, top: 241 },
  { name: "ci", left: 722, top: 433 },
  { name: "review-bot", left: 553, top: 595 },
  { name: "web", left: 352, top: 632 },
  { name: "cron", left: 151, top: 523 },
  { name: "maya-mbp", left: 87, top: 331 },
  { name: "deploy", left: 229, top: 169 },
];
</script>

<template>
  <div class="absolute inset-0" aria-hidden="true">
    <img :src="art" alt="" class="absolute inset-0 size-full" />
    <div
      class="absolute flex items-center justify-center"
      :style="{
        left: artPx(408),
        top: artPx(368),
        width: artPx(64),
        height: artPx(64),
      }">
      <DopbaseIcon class="h-[81.25%] w-auto" />
    </div>
    <p
      v-for="app in apps"
      :key="app.name"
      class="absolute whitespace-nowrap border border-line-strong bg-raised font-mono leading-[normal] text-ink"
      :style="{
        left: artPx(app.left),
        top: artPx(app.top),
        padding: `${artPx(7)} ${artPx(12)}`,
        borderRadius: artPx(8),
        fontSize: artPx(15),
      }">
      {{ app.name }}
    </p>
  </div>
</template>
