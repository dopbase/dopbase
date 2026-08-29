<script setup lang="ts">
import { computed, ref, watch } from "vue";
import DbModal from "./DbModal.vue";
import DbButton from "./DbButton.vue";
import DbInput from "./DbInput.vue";

/**
 * DbConfirmDialog — confirmation dialog for ordinary and destructive
 * actions. In destructive mode (`confirmWord` set) the user must type the
 * exact resource name before confirming, and optional affected counts are
 * listed up front.
 */
const props = withDefaults(
  defineProps<{
    open: boolean;
    title: string;
    description?: string;
    confirmLabel?: string;
    cancelLabel?: string;
    tone?: "danger" | "primary";
    /** When set, must be typed exactly to enable the confirm button. */
    confirmWord?: string;
    affectedCounts?: Array<{ label: string; count: number }>;
    loading?: boolean;
    error?: string | null;
  }>(),
  {
    confirmLabel: "Confirm",
    cancelLabel: "Cancel",
    tone: "danger",
  },
);

const emit = defineEmits<{ confirm: []; close: [] }>();

const typed = ref("");

watch(
  () => props.open,
  (open) => {
    if (open) typed.value = "";
  },
);

const canConfirm = computed(
  () => !props.confirmWord || typed.value === props.confirmWord,
);

const confirmPrompt = computed(() =>
  props.confirmWord ? `Type "${props.confirmWord}" to confirm` : undefined,
);
</script>

<template>
  <DbModal :open="open" :title="title" size="sm" @close="emit('close')">
    <div class="flex flex-col gap-3">
      <p v-if="description" class="text-sm text-ink">
        {{ description }}
      </p>

      <ul
        v-if="affectedCounts && affectedCounts.length > 0"
        class="flex flex-col gap-1 rounded-md border border-line-soft bg-canvas px-3 py-2 font-mono text-xs text-ink-muted">
        <li
          v-for="item in affectedCounts"
          :key="item.label"
          class="flex items-center justify-between gap-4">
          <span>{{ item.label }}</span>
          <span class="text-ink-strong">{{ item.count }}</span>
        </li>
      </ul>

      <DbInput
        v-if="confirmWord"
        v-model="typed"
        :label="confirmPrompt"
        :placeholder="confirmWord"
        mono />

      <p v-if="error" class="text-xs text-crit">
        {{ error }}
      </p>
    </div>

    <template #footer>
      <DbButton variant="ghost" :disabled="loading" @click="emit('close')">
        {{ cancelLabel }}
      </DbButton>
      <DbButton
        :variant="tone === 'danger' ? 'danger' : 'primary'"
        :loading="loading"
        :disabled="!canConfirm"
        @click="emit('confirm')">
        {{ confirmLabel }}
      </DbButton>
    </template>
  </DbModal>
</template>
