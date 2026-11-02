<script setup lang="ts">
import { ref } from "vue";
import { PlusIcon, UploadIcon } from "~/assets/icons";
import { DbButton } from "~/components/ui";

/**
 * EnvDropZone — every way to put secrets into an empty environment by
 * hand: drop or choose a .env file, paste one, or add a secret. Renders
 * props and reports what was picked.
 */
defineProps<{
  /** While a dropped file is being read. */
  importing: boolean;
  /** Whether Choose a file is the page's main action. */
  primary?: boolean;
}>();

const emit = defineEmits<{
  "choose-file": [];
  "drop-file": [file: File];
  "paste-env": [];
  "add-one": [];
}>();

const dragging = ref(false);

function onDrop(event: DragEvent): void {
  dragging.value = false;
  const file = event.dataTransfer?.files?.[0];
  if (file) emit("drop-file", file);
}
</script>

<template>
  <div
    class="flex flex-col items-center gap-2 rounded-card border border-dashed px-6 py-6 text-center transition-colors"
    :class="
      dragging ? 'border-accent/60 bg-accent-soft' : 'border-line bg-panel'
    "
    data-testid="env-drop-zone"
    @dragover.prevent="dragging = true"
    @dragleave.prevent="dragging = false"
    @drop.prevent="onDrop">
    <UploadIcon class="h-4 w-4 text-ink-muted" />
    <p class="text-sm font-medium leading-5 text-ink-strong">
      {{ importing ? "Reading the file…" : "Drop a .env file here" }}
    </p>
    <p class="text-sm leading-[22px] text-ink-muted">
      Values are encrypted before they're stored. The file isn't kept.
    </p>
    <div class="mt-1 flex flex-wrap justify-center gap-2">
      <DbButton
        :variant="primary ? 'primary' : 'secondary'"
        :disabled="importing"
        @click="emit('choose-file')">
        Choose a file
      </DbButton>
      <DbButton :disabled="importing" @click="emit('paste-env')">
        Paste .env
      </DbButton>
      <DbButton :disabled="importing" @click="emit('add-one')">
        <PlusIcon class="h-4 w-4" />
        Add secret
      </DbButton>
    </div>
  </div>
</template>
