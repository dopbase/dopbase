<script setup lang="ts">
import art from "~/assets/artwork/layers.svg";
import { LockIcon } from "~/assets/icons";
import { artPx } from "./AuthArtwork.controller";
</script>

<template>
  <div
    class="absolute inset-0 font-mono leading-[normal]"
    :style="{ fontSize: artPx(18) }"
    aria-hidden="true">
    <img :src="art" alt="" class="absolute inset-0 size-full" />
    <p
      class="absolute text-ink-muted"
      :style="{ left: artPx(198), top: artPx(656) }">
      dev
    </p>
    <p class="absolute text-ink" :style="{ left: artPx(253), top: artPx(571) }">
      staging
    </p>
    <LockIcon
      class="absolute text-warn"
      :style="{
        left: artPx(310),
        top: artPx(297),
        width: artPx(20),
        height: artPx(20),
      }" />
    <p
      class="absolute text-ink-strong"
      :style="{ left: artPx(340), top: artPx(294) }">
      prod
    </p>
  </div>
</template>
