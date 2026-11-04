<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { XIcon } from "~/assets/icons";
import { acquireScrollLock, releaseScrollLock } from "~/composable/scroll-lock";
import { modalsSuspended } from "~/composable/modal-suspension";

/**
 * DbModal — the single dialog primitive. Renders via Teleport, closes on
 * Escape and backdrop click (unless `persistent`), and locks body scroll
 * while open. Below `sm` it docks to the bottom of the screen; the body
 * scrolls on its own so the header and footer actions stay in reach.
 *
 * `layer` decides stacking, independent of mount order: page dialogs sit
 * below the password confirmation (`auth`), which sits below "Session
 * expired" (`session`). While the session is gone every dialog except the
 * session layer hides and asks its owner to close.
 */
const props = withDefaults(
  defineProps<{
    open: boolean;
    title: string;
    size?: "sm" | "md" | "lg" | "xl";
    persistent?: boolean;
    layer?: "page" | "auth" | "session";
  }>(),
  { size: "md", layer: "page" },
);

const emit = defineEmits<{ close: [] }>();

const panel = ref<HTMLElement | null>(null);

const widthClass = {
  sm: "max-w-sm",
  md: "max-w-lg",
  lg: "max-w-2xl",
  xl: "max-w-4xl",
}[props.size];

const layerClass = {
  page: "z-50",
  auth: "z-[60]",
  session: "z-[70]",
}[props.layer];

const visible = computed(
  () => props.open && !(modalsSuspended.value && props.layer !== "session"),
);

function onKeydown(event: KeyboardEvent): void {
  if (event.key === "Escape" && !props.persistent) emit("close");
}

function focusPanel(): void {
  requestAnimationFrame(() => {
    const autofocusTarget = panel.value?.querySelector<HTMLElement>(
      "[autofocus]:not([disabled])",
    );
    (autofocusTarget ?? panel.value)?.focus();
  });
}

watch(visible, (open) => {
  if (open) {
    document.addEventListener("keydown", onKeydown);
    acquireScrollLock();
    focusPanel();
  } else {
    document.removeEventListener("keydown", onKeydown);
    releaseScrollLock();
  }
});

// Ask the owner to close as well, so its draft and pending state reset. An
// owner that refuses (for example mid-save) stays hidden all the same.
watch(modalsSuspended, (suspended) => {
  if (suspended && props.open && props.layer !== "session") emit("close");
});

onMounted(() => {
  if (visible.value) {
    document.addEventListener("keydown", onKeydown);
    acquireScrollLock();
    focusPanel();
  }
});

onBeforeUnmount(() => {
  document.removeEventListener("keydown", onKeydown);
  if (visible.value) releaseScrollLock();
});
</script>

<template>
  <Teleport to="body">
    <div
      v-if="visible"
      :class="layerClass"
      class="fixed inset-0 flex items-end justify-center bg-black/60 pt-4 sm:items-center sm:p-4"
      @mousedown.self="!persistent && emit('close')">
      <div
        ref="panel"
        :class="widthClass"
        class="flex max-h-[calc(100svh-1rem)] w-full flex-col rounded-t-card border border-line bg-panel shadow-2xl focus:outline-none sm:max-h-[calc(100svh-2rem)] sm:rounded-card"
        role="dialog"
        aria-modal="true"
        :aria-label="title"
        tabindex="-1">
        <header
          class="flex shrink-0 items-center justify-between gap-3 border-b border-line-soft px-5 py-3.5">
          <h2
            class="min-w-0 text-[15px] font-semibold text-ink-strong [overflow-wrap:anywhere]">
            {{ title }}
          </h2>
          <button
            v-if="!persistent"
            type="button"
            class="cursor-pointer rounded-control p-1 text-ink-muted pointer-coarse:p-2 transition-colors hover:bg-raised hover:text-ink-strong"
            aria-label="Close dialog"
            @click="emit('close')">
            <XIcon class="h-4 w-4" />
          </button>
        </header>
        <div class="min-h-0 overflow-y-auto px-5 py-4">
          <slot />
        </div>
        <footer
          v-if="$slots.footer"
          class="flex shrink-0 flex-wrap items-center justify-end gap-2 border-t border-line-soft px-5 py-3.5">
          <slot name="footer" />
        </footer>
      </div>
    </div>
  </Teleport>
</template>
