<script setup lang="ts">
import { inject } from "vue";
import { menuCloseKey } from "./menu";

/**
 * DbMenuItem — one action in a `DbMenu`. Emits `select` and closes the
 * menu. The danger tone is for destructive actions, which go last.
 * Presentation-only.
 */
withDefaults(
  defineProps<{
    tone?: "default" | "danger";
    disabled?: boolean;
  }>(),
  { tone: "default", disabled: false },
);

const emit = defineEmits<{ select: [] }>();
const closeMenu = inject(menuCloseKey, () => {});

function choose(): void {
  closeMenu();
  emit("select");
}
</script>

<template>
  <button
    type="button"
    role="menuitem"
    tabindex="-1"
    :disabled="disabled"
    class="flex h-9 w-full cursor-pointer items-center gap-2.5 whitespace-nowrap rounded-control px-2.5 text-left text-sm font-medium leading-5 outline-none transition-colors focus-visible:bg-raised disabled:cursor-not-allowed disabled:opacity-50 pointer-coarse:min-h-11"
    :class="
      tone === 'danger'
        ? 'text-crit hover:bg-crit/10 focus-visible:bg-crit/10'
        : 'text-ink hover:bg-raised hover:text-ink-strong focus-visible:text-ink-strong'
    "
    @click="choose">
    <span
      v-if="$slots.icon"
      class="flex h-4 w-4 shrink-0 items-center justify-center"
      :class="tone === 'danger' ? 'text-crit' : 'text-ink-muted'"
      aria-hidden="true">
      <slot name="icon" />
    </span>
    <span class="min-w-0 flex-1 truncate"><slot /></span>
    <slot name="trailing" />
  </button>
</template>
