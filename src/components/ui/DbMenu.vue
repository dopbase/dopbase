<script setup lang="ts">
import {
  nextTick,
  onBeforeUnmount,
  provide,
  ref,
  useId,
  watch,
  type CSSProperties,
} from "vue";
import { MoreHorizontalIcon } from "~/assets/icons";
import { menuCloseKey } from "./menu";

/**
 * DbMenu — a dropdown of actions. By default the trigger is a 32px ⋯
 * button named by `label`; pass a `trigger` slot for anything else and bind
 * its `attrs` and `toggle`. The list renders in a Teleport with fixed
 * positioning, so a table's overflow never clips it, and flips above the
 * trigger near the bottom of the screen.
 *
 * Arrow keys move between items, Escape closes and returns focus, and an
 * outside click, scroll or resize closes it. `DbMenuItem` closes it when
 * chosen. Presentation-only: items report what was picked.
 */
const props = withDefaults(
  defineProps<{
    /** Names the default ⋯ trigger. */
    label?: string;
    align?: "start" | "end";
  }>(),
  { label: "More actions", align: "end" },
);

const menuId = useId();
const open = ref(false);
const anchor = ref<HTMLElement | null>(null);
const panel = ref<HTMLElement | null>(null);
const position = ref<CSSProperties>({});

function items(): HTMLElement[] {
  return Array.from(
    panel.value?.querySelectorAll<HTMLElement>(
      ':is([role="menuitem"], [role="menuitemradio"]):not([disabled])',
    ) ?? [],
  );
}

function place(): void {
  const rect = anchor.value?.getBoundingClientRect();
  if (!rect) return;
  const height = panel.value?.offsetHeight ?? 0;
  const below = rect.bottom + 4;
  const top =
    below + height > window.innerHeight - 8 && rect.top - height - 4 > 8
      ? rect.top - height - 4
      : below;
  // Keep the list on screen: on a phone a start-aligned menu can be wider
  // than the space to the right of its trigger.
  const width = panel.value?.offsetWidth ?? 0;
  const margin = 8;
  const left =
    props.align === "end"
      ? Math.max(margin, rect.right - width)
      : Math.min(rect.left, window.innerWidth - width - margin);
  position.value = { top: `${top}px`, left: `${Math.max(margin, left)}px` };
}

function close(returnFocus = false): void {
  if (!open.value) return;
  open.value = false;
  // Wait for the list to leave the page, so removing the focused item
  // can't drop focus after it has moved back to the trigger.
  if (returnFocus)
    void nextTick(() =>
      anchor.value
        ?.querySelector<HTMLElement>("button, a, [tabindex]")
        ?.focus(),
    );
}

function toggle(): void {
  open.value = !open.value;
}

function onPointerDown(event: MouseEvent): void {
  const target = event.target as Node;
  if (anchor.value?.contains(target) || panel.value?.contains(target)) return;
  close();
}

function onKeydown(event: KeyboardEvent): void {
  if (event.key === "Escape") {
    event.preventDefault();
    close(true);
    return;
  }
  if (event.key === "Tab") {
    close();
    return;
  }
  if (event.key !== "ArrowDown" && event.key !== "ArrowUp") return;
  event.preventDefault();
  const list = items();
  if (list.length === 0) return;
  const current = list.indexOf(document.activeElement as HTMLElement);
  const step = event.key === "ArrowDown" ? 1 : -1;
  list[(current + step + list.length) % list.length]?.focus();
}

function onViewportChange(): void {
  close();
}

/**
 * The list is fixed to where its trigger was, so it closes when the page
 * scrolls. Scrolling the list itself, by wheel, touch, or arrow keys
 * moving focus, keeps it open.
 */
function onScroll(event: Event): void {
  const target = event.target;
  if (target instanceof Node && panel.value?.contains(target)) return;
  close();
}

function listen(on: boolean): void {
  if (on) {
    document.addEventListener("mousedown", onPointerDown);
    document.addEventListener("keydown", onKeydown);
    window.addEventListener("scroll", onScroll, true);
    window.addEventListener("resize", onViewportChange);
    return;
  }
  document.removeEventListener("mousedown", onPointerDown);
  document.removeEventListener("keydown", onKeydown);
  window.removeEventListener("scroll", onScroll, true);
  window.removeEventListener("resize", onViewportChange);
}

watch(open, async (isOpen) => {
  listen(isOpen);
  if (!isOpen) return;
  place();
  await nextTick();
  place();
  // A pick-one menu opens on its current choice, like a native select.
  const list = items();
  (
    list.find((item) => item.getAttribute("aria-checked") === "true") ?? list[0]
  )?.focus();
});

onBeforeUnmount(() => listen(false));

provide(menuCloseKey, () => close(true));
</script>

<template>
  <span ref="anchor" class="inline-flex min-w-0 max-w-full">
    <slot
      name="trigger"
      :open="open"
      :toggle="toggle"
      :attrs="{
        'aria-haspopup': 'menu' as const,
        'aria-expanded': open,
        'aria-controls': open ? menuId : undefined,
      }">
      <button
        type="button"
        class="flex h-8 w-8 cursor-pointer items-center justify-center rounded-control text-ink-muted transition-colors hover:bg-raised hover:text-ink-strong pointer-coarse:h-11 pointer-coarse:w-11"
        :class="open ? 'bg-raised text-ink-strong' : ''"
        :aria-label="label"
        :title="label"
        aria-haspopup="menu"
        :aria-expanded="open"
        :aria-controls="open ? menuId : undefined"
        @click="toggle">
        <MoreHorizontalIcon class="h-4 w-4" />
      </button>
    </slot>
  </span>
  <Teleport to="body">
    <div
      v-if="open"
      :id="menuId"
      ref="panel"
      class="fixed z-40 flex max-h-[min(24rem,70vh)] min-w-48 max-w-[min(20rem,calc(100vw-1rem))] flex-col gap-0.5 overflow-y-auto overscroll-contain rounded-card border border-line bg-panel p-1 shadow-xl"
      :style="position"
      role="menu"
      data-testid="menu">
      <slot :close="close" />
    </div>
  </Teleport>
</template>
