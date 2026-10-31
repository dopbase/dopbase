<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { XIcon } from "~/assets/icons";
import { acquireScrollLock, releaseScrollLock } from "~/composable/scroll-lock";

/**
 * DbSheet — a panel that slides in over the page. From the left it is a
 * full-height column, used where a desktop column (the app sidebar) folds
 * away on narrow screens. From the bottom it is a phone picker, used where
 * a desktop dropdown would be too small to tap. Renders via Teleport,
 * closes on Escape and backdrop click, and shares the body scroll lock
 * with DbModal.
 */
const props = withDefaults(
  defineProps<{
    open: boolean;
    title: string;
    side?: "left" | "bottom";
  }>(),
  { side: "left" },
);

const emit = defineEmits<{ close: [] }>();

const panel = ref<HTMLElement | null>(null);

const bottom = computed(() => props.side === "bottom");
/** Where the panel slides from, for the enter and leave transitions. */
const offscreen = computed(() =>
  bottom.value
    ? "opacity-0 [&>[role=dialog]]:translate-y-full motion-reduce:[&>[role=dialog]]:translate-y-0"
    : "opacity-0 [&>[role=dialog]]:-translate-x-full motion-reduce:[&>[role=dialog]]:translate-x-0",
);

function onKeydown(event: KeyboardEvent): void {
  if (event.key === "Escape") emit("close");
}

function activate(): void {
  document.addEventListener("keydown", onKeydown);
  acquireScrollLock();
  requestAnimationFrame(() => panel.value?.focus());
}

function deactivate(): void {
  document.removeEventListener("keydown", onKeydown);
  releaseScrollLock();
}

watch(
  () => props.open,
  (open) => (open ? activate() : deactivate()),
);

onMounted(() => {
  if (props.open) activate();
});

onBeforeUnmount(() => {
  document.removeEventListener("keydown", onKeydown);
  if (props.open) releaseScrollLock();
});
</script>

<template>
  <Teleport to="body">
    <Transition
      enter-active-class="transition-opacity duration-200 motion-reduce:transition-none [&>[role=dialog]]:transition-transform [&>[role=dialog]]:duration-200 [&>[role=dialog]]:ease-out"
      leave-active-class="transition-opacity duration-150 motion-reduce:transition-none [&>[role=dialog]]:transition-transform [&>[role=dialog]]:duration-150 [&>[role=dialog]]:ease-in"
      :enter-from-class="offscreen"
      :leave-to-class="offscreen">
      <div
        v-if="open"
        class="fixed inset-0 z-50 flex bg-black/60"
        :class="bottom && 'items-end'"
        @mousedown.self="emit('close')">
        <div
          ref="panel"
          class="flex flex-col bg-panel shadow-2xl focus:outline-none"
          :class="
            bottom
              ? 'max-h-[85dvh] w-full rounded-t-card border-t border-line pb-[env(safe-area-inset-bottom)]'
              : 'h-full w-[min(20rem,85vw)] border-r border-line'
          "
          role="dialog"
          aria-modal="true"
          :aria-label="title"
          tabindex="-1">
          <header
            class="flex h-12 shrink-0 items-center justify-between gap-3 border-b border-line-soft pl-4 pr-2">
            <h2 class="truncate text-sm font-semibold text-ink-strong">
              {{ title }}
            </h2>
            <button
              type="button"
              class="flex h-11 w-11 cursor-pointer items-center justify-center rounded-control text-ink-muted transition-colors hover:bg-raised hover:text-ink-strong"
              aria-label="Close"
              @click="emit('close')">
              <XIcon class="h-4 w-4" />
            </button>
          </header>
          <div class="flex min-h-0 flex-1 flex-col">
            <slot />
          </div>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>
