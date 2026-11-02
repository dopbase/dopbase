<script setup lang="ts">
import art from "~/assets/artwork/dial.svg";
import { DopbaseIcon } from "~/assets/icons";
import { artPx } from "./AuthArtwork.controller";
</script>

<template>
  <div class="absolute inset-0" aria-hidden="true">
    <img :src="art" alt="" class="absolute inset-0 size-full" />
    <div
      class="absolute flex items-center justify-center"
      :style="{
        left: artPx(394),
        top: artPx(354),
        width: artPx(92),
        height: artPx(92),
      }">
      <DopbaseIcon class="h-[81.25%] w-auto" />
    </div>
  </div>
</template>
