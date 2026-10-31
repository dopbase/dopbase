<script setup lang="ts">
import { computed } from "vue";
import { AlertTriangleIcon, CheckIcon, InfoIcon } from "~/assets/icons";

/**
 * DbAlert — inline banner for form-level and load errors, and notes.
 * With a `title`, it shows the tone's icon and the title, and the slot
 * becomes the description below it. Presentation-only.
 */
const props = withDefaults(
  defineProps<{
    tone?: "error" | "info" | "success";
    title?: string;
  }>(),
  { tone: "error", title: undefined },
);

const icon = computed(() => {
  switch (props.tone) {
    case "info":
      return InfoIcon;
    case "success":
      return CheckIcon;
    default:
      return AlertTriangleIcon;
  }
});

/** Only errors interrupt a screen reader; notes are read in place. */
const role = computed(() => {
  switch (props.tone) {
    case "info":
      return "note";
    case "success":
      return "status";
    default:
      return "alert";
  }
});

const toneClasses = computed(() => {
  switch (props.tone) {
    case "info":
      return "border-accent/30 bg-accent-soft text-accent-strong";
    case "success":
      return "border-ok/30 bg-ok/10 text-ok";
    default:
      return "border-crit/30 bg-crit/10 text-crit";
  }
});
</script>

<template>
  <div
    class="rounded-control border px-3.5 py-2.5 text-xs leading-relaxed"
    :class="toneClasses"
    :role="role">
    <div v-if="title" class="flex items-start gap-2.5">
      <component
        :is="icon"
        class="mt-0.5 h-4 w-4 shrink-0"
        aria-hidden="true" />
      <div class="min-w-0">
        <p class="text-sm font-semibold leading-5 text-ink-strong">
          {{ title }}
        </p>
        <div class="mt-0.5 text-[13px] leading-5 text-ink">
          <slot />
        </div>
      </div>
    </div>
    <slot v-else />
  </div>
</template>
