<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { CheckIcon, FolderIcon, LayersIcon, LockIcon } from "~/assets/icons";
import { DbSearchField, DbSheet } from "~/components/ui";

export interface SwitcherItem {
  id: string;
  name: string;
  /** Shown on the right, such as the viewer's level there. */
  detail?: string;
  protected?: boolean;
}

/**
 * SwitcherSheet — the project and environment pickers on phones. A bottom
 * sheet with rows big enough to tap, the same details the desktop menus
 * show, and a filter once the list is long enough to need one.
 */
const props = defineProps<{
  open: boolean;
  title: string;
  kind: "project" | "environment";
  items: SwitcherItem[] | null;
  currentId: string | null;
}>();

const emit = defineEmits<{ select: [id: string]; close: [] }>();

/** Lists longer than this get a filter. */
const FILTER_AFTER = 7;

const filter = ref("");
watch(
  () => props.open,
  (open) => {
    if (open) filter.value = "";
  },
);

const visible = computed(() => {
  const needle = filter.value.trim().toLowerCase();
  const all = props.items ?? [];
  return needle
    ? all.filter((item) => item.name.toLowerCase().includes(needle))
    : all;
});

function choose(id: string): void {
  emit("select", id);
  emit("close");
}
</script>

<template>
  <DbSheet :open="open" :title="title" side="bottom" @close="emit('close')">
    <div
      v-if="(items?.length ?? 0) > FILTER_AFTER"
      class="shrink-0 border-b border-line-soft px-4 py-3">
      <DbSearchField
        v-model="filter"
        :placeholder="`Filter ${kind === 'project' ? 'projects' : 'environments'}`" />
    </div>
    <ul
      class="min-h-0 flex-1 overflow-y-auto overscroll-contain py-1"
      :aria-label="title"
      data-testid="switcher-sheet">
      <li v-for="item in visible" :key="item.id">
        <button
          type="button"
          class="flex min-h-12 w-full cursor-pointer items-center gap-3 px-4 py-2 text-left transition-colors hover:bg-raised active:bg-raised"
          :aria-current="item.id === currentId ? 'true' : undefined"
          @click="choose(item.id)">
          <LockIcon v-if="item.protected" class="h-4 w-4 shrink-0 text-warn" />
          <FolderIcon
            v-else-if="kind === 'project'"
            class="h-4 w-4 shrink-0 text-ink-muted" />
          <LayersIcon v-else class="h-4 w-4 shrink-0 text-ink-muted" />
          <span class="min-w-0 flex-1">
            <span
              class="block truncate font-mono text-sm font-medium leading-[22px] text-ink-strong">
              {{ item.name }}
            </span>
            <span
              v-if="item.protected"
              class="block text-[13px] leading-5 text-warn">
              Protected
            </span>
          </span>
          <span
            v-if="item.detail"
            class="shrink-0 text-[13px] leading-5 text-ink-muted">
            {{ item.detail }}
          </span>
          <CheckIcon
            class="h-4 w-4 shrink-0"
            :class="item.id === currentId ? 'text-accent-strong' : 'invisible'"
            aria-hidden="true" />
        </button>
      </li>
      <li v-if="items === null" class="px-4 py-3 text-sm text-ink-muted">
        Loading…
      </li>
      <li
        v-else-if="visible.length === 0"
        class="px-4 py-3 text-sm text-ink-muted">
        Nothing matches "{{ filter.trim() }}".
      </li>
    </ul>
  </DbSheet>
</template>
