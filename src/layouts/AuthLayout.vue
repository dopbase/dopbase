<script setup lang="ts">
import { DopbaseIcon } from "~/assets/icons";

/**
 * AuthLayout — split screen for sign in, setup and CLI approval.
 *
 * Left: the logo, the form slot in the middle and a `footer` slot at the
 * bottom. Right: an `aside` slot; without one it shows the instance seal
 * art and the tagline. The right side drops on phones.
 */
</script>

<template>
  <div class="flex min-h-svh bg-canvas">
    <div
      class="flex min-h-svh w-full flex-col px-6 py-8 sm:px-12 sm:py-10 lg:w-[560px] lg:shrink-0 lg:border-r lg:border-line-soft lg:px-16">
      <div class="flex items-center gap-2">
        <DopbaseIcon class="h-6 w-6 shrink-0" />
        <span class="text-[15px] font-semibold text-ink-strong">Dopbase</span>
      </div>

      <main class="flex flex-1 items-center py-10">
        <div class="w-full max-w-[432px]">
          <slot />
        </div>
      </main>

      <footer
        v-if="$slots.footer"
        class="flex flex-wrap items-center justify-between gap-x-4 gap-y-2 text-[13px] text-ink-muted">
        <slot name="footer" />
      </footer>
    </div>

    <aside class="relative hidden flex-1 overflow-hidden bg-panel lg:flex">
      <slot name="aside">
        <div
          class="flex flex-1 flex-col items-center justify-center gap-10 p-12">
          <div class="relative w-full max-w-[600px]">
            <svg
              viewBox="0 0 600 600"
              class="h-auto w-full max-w-[600px]"
              aria-hidden="true">
              <defs>
                <radialGradient id="auth-glow" cx="50%" cy="50%" r="50%">
                  <stop offset="0%" stop-color="#9b30f0" stop-opacity="0.38" />
                  <stop offset="70%" stop-color="#9b30f0" stop-opacity="0.06" />
                  <stop offset="100%" stop-color="#9b30f0" stop-opacity="0" />
                </radialGradient>
              </defs>
              <rect
                x="0"
                y="0"
                width="600"
                height="600"
                fill="url(#auth-glow)" />
              <g fill="none" stroke="#c4a1ff">
                <rect
                  v-for="ring in 7"
                  :key="ring"
                  :x="300 - (ring * 40 + 20)"
                  :y="300 - (ring * 40 + 20)"
                  :width="(ring * 40 + 20) * 2"
                  :height="(ring * 40 + 20) * 2"
                  :rx="16 + ring * 10"
                  :stroke-opacity="0.55 - ring * 0.06" />
              </g>
              <g fill="#c4a1ff" fill-opacity="0.7">
                <circle cx="300" cy="150" r="3.5" />
                <circle cx="300" cy="450" r="3.5" />
                <circle cx="150" cy="300" r="3.5" />
                <circle cx="450" cy="300" r="3.5" />
                <circle cx="210" cy="210" r="3.5" />
                <circle cx="390" cy="210" r="3.5" />
                <circle cx="210" cy="390" r="3.5" />
                <circle cx="390" cy="390" r="3.5" />
              </g>
            </svg>
            <DopbaseIcon
              class="absolute left-1/2 top-1/2 h-[72px] w-[72px] -translate-x-1/2 -translate-y-1/2" />
          </div>
          <p class="text-2xl font-semibold text-ink-strong">
            Encrypted before it's stored.
          </p>
        </div>
      </slot>
    </aside>
  </div>
</template>
