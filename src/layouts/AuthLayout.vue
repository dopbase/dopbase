<script setup lang="ts">
import { useServerStatus } from "~/composable";
import { DopbaseIcon } from "~/assets/icons";

/**
 * AuthLayout — split screen for `/login` and `/setup`.
 *
 * Left: the dark "instance seal" panel showing only the endpoint and
 * server reachability. Right: the form slot. On mobile the seal collapses
 * into a compact header above the form.
 */
const { health, reachable, endpoint } = useServerStatus();
</script>

<template>
  <div class="flex min-h-svh flex-col md:flex-row">
    <!-- Instance seal -->
    <aside
      class="flex flex-col justify-between border-b border-line bg-panel px-8 py-8 md:w-[380px] md:border-b-0 md:border-r lg:w-[440px] lg:px-12 lg:py-12">
      <div class="flex items-center gap-3">
        <div
          class="flex h-10 w-10 items-center justify-center rounded-lg border border-accent/40 bg-accent-soft text-accent-strong">
          <DopbaseIcon class="h-6 w-6" />
        </div>
        <div>
          <p class="font-mono text-sm font-semibold text-ink-strong">dopbase</p>
          <p class="text-xs text-ink-muted">self-hosted secret manager</p>
        </div>
      </div>

      <div class="mt-10 md:mt-0">
        <dl class="flex flex-col gap-3 font-mono text-xs">
          <div class="flex items-center justify-between gap-4">
            <dt class="text-ink-muted">endpoint</dt>
            <dd class="truncate text-ink-strong">{{ endpoint }}</dd>
          </div>
          <div class="flex items-center justify-between gap-4">
            <dt class="text-ink-muted">server</dt>
            <dd class="flex items-center gap-2">
              <span
                class="inline-block h-2 w-2 rounded-full"
                :class="
                  reachable === null
                    ? 'bg-ink-faint'
                    : reachable
                      ? 'bg-ok'
                      : 'bg-crit'
                " />
              <span
                :class="
                  reachable === null
                    ? 'text-ink-muted'
                    : reachable
                      ? 'text-ok'
                      : 'text-crit'
                ">
                {{
                  reachable === null
                    ? "checking…"
                    : reachable
                      ? "reachable"
                      : "unreachable"
                }}
              </span>
            </dd>
          </div>
          <div v-if="health" class="flex items-center justify-between gap-4">
            <dt class="text-ink-muted">version</dt>
            <dd class="text-ink-strong">v{{ health.version }}</dd>
          </div>
        </dl>
      </div>

      <p
        class="mt-10 hidden text-xs leading-relaxed text-ink-faint md:mt-0 md:block">
        Secrets are encrypted at rest with the server master key. The key never
        enters the browser.
      </p>
    </aside>

    <!-- Form side -->
    <main
      class="auth-form-side relative flex flex-1 items-start justify-center px-6 py-10 md:items-center md:px-12">
      <slot />
    </main>
  </div>
</template>

<style scoped>
/* Blueprint grid on the form side, fading out from the center so the
   form stays the focal point. Painted behind the slot content. */
.auth-form-side::before {
  content: "";
  position: absolute;
  inset: 0;
  z-index: -1;
  background-image:
    linear-gradient(to right, rgb(35 40 55 / 0.55) 1px, transparent 1px),
    linear-gradient(to bottom, rgb(35 40 55 / 0.55) 1px, transparent 1px);
  background-size: 32px 32px;
  mask-image: radial-gradient(
    ellipse 70% 60% at 50% 45%,
    black 20%,
    transparent 80%
  );
  -webkit-mask-image: radial-gradient(
    ellipse 70% 60% at 50% 45%,
    black 20%,
    transparent 80%
  );
}
</style>
