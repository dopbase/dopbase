<script setup lang="ts">
import {
  nextTick,
  onBeforeUnmount,
  ref,
  useId,
  watch,
  type CSSProperties,
} from "vue";
import { InfoIcon } from "~/assets/icons";

/**
 * DbInfoTip — a small ⓘ button that explains the thing beside it. The tip
 * shows on hover, keyboard focus, or a tap on touch screens. It renders in a Teleport with fixed positioning, so a card's
 * overflow never clips it, stays on screen near the edges, and flips
 * above the button near the bottom. Escape, an outside tap, scroll or
 * resize hides it. Presentation-only.
 */
const props = withDefaults(
  defineProps<{
    /** The explanation. */
    text: string;
    /** Names the button for screen readers. */
    label?: string;
  }>(),
  { label: "More info" },
);

const tipId = useId();
const open = ref(false);
const anchor = ref<HTMLElement | null>(null);
const panel = ref<HTMLElement | null>(null);
const position = ref<CSSProperties>({});

function place(): void {
  const rect = anchor.value?.getBoundingClientRect();
  if (!rect) return;
  const margin = 8;
  const width = panel.value?.offsetWidth ?? 0;
  const height = panel.value?.offsetHeight ?? 0;
  const below = rect.bottom + 6;
  const top =
    below + height > window.innerHeight - margin &&
    rect.top - height - 6 > margin
      ? rect.top - height - 6
      : below;
  const centered = rect.left + rect.width / 2 - width / 2;
  const left = Math.min(
    Math.max(margin, centered),
    window.innerWidth - width - margin,
  );
  position.value = { top: `${top}px`, left: `${Math.max(margin, left)}px` };
}

function show(): void {
  open.value = true;
}

function hide(): void {
  open.value = false;
}

function onPointerDown(event: Event): void {
  if (anchor.value?.contains(event.target as Node)) return;
  hide();
}

function onKeydown(event: KeyboardEvent): void {
  if (event.key === "Escape") hide();
}

function listen(on: boolean): void {
  if (on) {
    document.addEventListener("pointerdown", onPointerDown);
    document.addEventListener("keydown", onKeydown);
    window.addEventListener("scroll", hide, true);
    window.addEventListener("resize", hide);
    return;
  }
  document.removeEventListener("pointerdown", onPointerDown);
  document.removeEventListener("keydown", onKeydown);
  window.removeEventListener("scroll", hide, true);
  window.removeEventListener("resize", hide);
}

watch(open, async (isOpen) => {
  listen(isOpen);
  if (!isOpen) return;
  place();
  await nextTick();
  place();
});

onBeforeUnmount(() => listen(false));
</script>

<template>
  <span
    ref="anchor"
    class="inline-flex shrink-0"
    @mouseenter="show"
    @mouseleave="hide">
    <button
      type="button"
      class="flex h-4 w-4 cursor-help items-center justify-center rounded-full text-ink-faint transition-colors hover:text-ink-strong focus-visible:text-ink-strong pointer-coarse:h-6 pointer-coarse:w-6"
      :aria-label="props.label"
      :aria-describedby="open ? tipId : undefined"
      @focus="show"
      @blur="hide"
      @click="show">
      <InfoIcon class="h-3.5 w-3.5" />
    </button>
  </span>
  <Teleport to="body">
    <div
      v-if="open"
      :id="tipId"
      ref="panel"
      class="pointer-events-none fixed z-50 max-w-[min(16rem,calc(100vw-1rem))] rounded-control border border-line bg-panel px-3 py-2 text-xs leading-5 text-ink shadow-xl"
      :style="position"
      role="tooltip">
      {{ props.text }}
    </div>
  </Teleport>
</template>
