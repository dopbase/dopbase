<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref, watch } from "vue";
import { XIcon } from "~/assets/icons";

/**
 * DbModal — the single dialog primitive. Renders via Teleport, closes on
 * Escape and backdrop click (unless `persistent`), and locks body scroll
 * while open.
 */
const props = withDefaults(
  defineProps<{
    open: boolean;
    title: string;
    size?: "sm" | "md" | "lg";
    persistent?: boolean;
  }>(),
  { size: "md" },
);

const emit = defineEmits<{ close: [] }>();

const panel = ref<HTMLElement | null>(null);

const widthClass = {
  sm: "max-w-sm",
  md: "max-w-lg",
  lg: "max-w-2xl",
}[props.size];

function onKeydown(event: KeyboardEvent): void {
  if (event.key === "Escape" && !props.persistent) emit("close");
}

watch(
  () => props.open,
  (open) => {
    if (open) {
      document.addEventListener("keydown", onKeydown);
      document.body.style.overflow = "hidden";
      requestAnimationFrame(() => panel.value?.focus());
    } else {
      document.removeEventListener("keydown", onKeydown);
      document.body.style.overflow = "";
    }
  },
);

onMounted(() => {
  if (props.open) {
    document.addEventListener("keydown", onKeydown);
    document.body.style.overflow = "hidden";
  }
});

onBeforeUnmount(() => {
  document.removeEventListener("keydown", onKeydown);
  document.body.style.overflow = "";
});
</script>

<template>
  <Teleport to="body">
    <div
      v-if="open"
      class="fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-4"
      @mousedown.self="!persistent && emit('close')">
      <div
        ref="panel"
        :class="widthClass"
        class="w-full rounded-[var(--radius-card)] border border-line bg-panel shadow-2xl focus:outline-none"
        role="dialog"
        aria-modal="true"
        :aria-label="title"
        tabindex="-1">
        <header
          class="flex items-center justify-between border-b border-line-soft px-5 py-3.5">
          <h2 class="text-[15px] font-semibold text-ink-strong">{{ title }}</h2>
          <button
            v-if="!persistent"
            type="button"
            class="cursor-pointer rounded p-1 text-ink-muted transition-colors hover:bg-raised hover:text-ink-strong"
            aria-label="Close dialog"
            @click="emit('close')">
            <XIcon class="h-4 w-4" />
          </button>
        </header>
        <div class="px-5 py-4">
          <slot />
        </div>
        <footer
          v-if="$slots.footer"
          class="flex items-center justify-end gap-2 border-t border-line-soft px-5 py-3.5">
          <slot name="footer" />
        </footer>
      </div>
    </div>
  </Teleport>
</template>
