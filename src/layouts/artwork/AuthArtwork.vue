<script setup lang="ts">
import type { Component } from "vue";
import DialArt from "./DialArt.vue";
import LayersArt from "./LayersArt.vue";
import MaskedArt from "./MaskedArt.vue";
import NetworkArt from "./NetworkArt.vue";
import VaultArt from "./VaultArt.vue";
import { artPx, useAuthArtworkController } from "./AuthArtwork.controller";
import type { ArtworkName } from "./AuthArtwork.controller";

/**
 * AuthArtwork — the picture and caption beside the sign-in form. A new one
 * is picked on every visit. It keeps its 880×900 frame and scales with the
 * aside, capped at the viewport height.
 */
const { artwork } = useAuthArtworkController();

const art: Record<ArtworkName, Component> = {
  vault: VaultArt,
  masked: MaskedArt,
  dial: DialArt,
  layers: LayersArt,
  network: NetworkArt,
};
</script>

<template>
  <div
    class="@container relative aspect-[880/900] w-[min(100%,880px,calc(100svh*880/900))]"
    :data-artwork="artwork.name"
    data-testid="auth-artwork">
    <component :is="art[artwork.name]" />
    <p
      class="absolute whitespace-nowrap font-semibold leading-[normal] text-ink-strong"
      :style="{
        left: artPx(80),
        top: artPx(788),
        fontSize: artPx(30),
        letterSpacing: artPx(-0.6),
      }">
      {{ artwork.caption }}
    </p>
  </div>
</template>
