<script setup lang="ts">
import { nextTick, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { useData } from "vitepress";

const { frontmatter } = useData();
const viewport = ref<HTMLDivElement | null>(null);
const dialog = ref<HTMLDialogElement | null>(null);
const overflowing = ref(false);
const open = ref(false);
const previousFocus = ref<HTMLElement | null>(null);
let resizeObserver: ResizeObserver | null = null;

function updateOverflow(): void {
  const element = viewport.value;
  overflowing.value =
    element !== null && element.scrollWidth > element.clientWidth + 1;
}

function observeTable(): void {
  resizeObserver?.disconnect();
  const container = viewport.value;
  const table = container?.querySelector<HTMLTableElement>("table") ?? null;
  if (container === null || table === null) return;

  resizeObserver = new ResizeObserver(updateOverflow);
  resizeObserver.observe(container);
  resizeObserver.observe(table);
  updateOverflow();
  if (document.fonts?.ready) void document.fonts.ready.then(updateOverflow);
}

function openViewer(): void {
  if (!overflowing.value) return;
  previousFocus.value = document.activeElement as HTMLElement | null;
  open.value = true;
  void nextTick(() => {
    if (dialog.value && !dialog.value.open) dialog.value.showModal();
  });
}

function closeViewer(): void {
  open.value = false;
  if (dialog.value?.open) dialog.value.close();
  void nextTick(() => {
    observeTable();
    previousFocus.value?.focus();
    previousFocus.value = null;
  });
}

function handleCancel(event: Event): void {
  event.preventDefault();
  closeViewer();
}

function handleBackdropClick(event: MouseEvent): void {
  if (event.target === event.currentTarget) closeViewer();
}

function handleDialogClose(): void {
  if (open.value) closeViewer();
}

watch(open, () => void nextTick(updateOverflow));
onMounted(observeTable);
onBeforeUnmount(() => resizeObserver?.disconnect());
</script>

<template>
  <div class="table-viewer">
    <div v-if="overflowing" class="table-viewer__toolbar">
      <button type="button" @click="openViewer">Expand table</button>
    </div>
    <div
      ref="viewport"
      class="table-viewer__viewport table-viewer__viewport--inline">
      <slot />
    </div>

    <Teleport to="body">
      <dialog
        v-if="open"
        ref="dialog"
        class="table-viewer__dialog"
        aria-label="Expanded table"
        @cancel="handleCancel"
        @close="handleDialogClose"
        @click="handleBackdropClick">
        <div
          class="table-viewer__content vp-doc"
          :class="frontmatter.pageClass">
          <header v-if="open" class="table-viewer__header">
            <span>Full table view</span>
            <button type="button" @click="closeViewer">Close</button>
          </header>
          <div class="table-viewer__viewport table-viewer__viewport--dialog">
            <slot />
          </div>
        </div>
      </dialog>
    </Teleport>
  </div>
</template>
