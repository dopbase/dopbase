<script setup lang="ts">
import art from "~/assets/artwork/vault.svg";
import { DopbaseIcon } from "~/assets/icons";
import { artPx } from "./AuthArtwork.controller";
</script>

<template>
  <div class="absolute inset-0" aria-hidden="true">
    <img :src="art" alt="" class="absolute inset-0 size-full" />
    <div
      class="absolute flex items-center justify-center"
      :style="{
        left: artPx(388),
        top: artPx(348),
        width: artPx(104),
        height: artPx(104),
      }">
      <DopbaseIcon class="h-[81.25%] w-auto" />
    </div>
  </div>
</template>
