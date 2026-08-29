<script setup lang="ts">
import { ref, watch } from "vue";
import DbModal from "~/components/ui/DbModal.vue";
import DbInput from "~/components/ui/DbInput.vue";
import DbButton from "~/components/ui/DbButton.vue";
import { ApiError } from "~/services/http.client";

/**
 * NameDialog — create/rename modal for projects and environments.
 * Runs the injected `action` and maps stable server error codes to
 * field-level messages.
 */
const props = withDefaults(
  defineProps<{
    open: boolean;
    title: string;
    label: string;
    initialName?: string;
    submitLabel?: string;
    hint?: string;
    action: (name: string) => Promise<void>;
  }>(),
  { initialName: "", submitLabel: "Save" },
);

const emit = defineEmits<{ close: []; success: [] }>();

const name = ref("");
const error = ref<string | null>(null);
const submitting = ref(false);

watch(
  () => props.open,
  (open) => {
    if (open) {
      name.value = props.initialName;
      error.value = null;
    }
  },
);

function mapError(cause: unknown): void {
  if (cause instanceof ApiError) {
    if (cause.status === 409) {
      error.value = "This name is already taken.";
      return;
    }
    if (
      cause.hasCode("REQUEST_INVALID") ||
      cause.hasCode("ENVIRONMENT_NAME_INVALID")
    ) {
      error.value = "Use 1–64 characters: letters, numbers, '-', '_'.";
      return;
    }
  }
  error.value = "The operation failed. Try again.";
}

async function submit(): Promise<void> {
  const trimmed = name.value.trim();
  if (trimmed === "") {
    error.value = "Enter a name.";
    return;
  }
  submitting.value = true;
  error.value = null;
  try {
    await props.action(trimmed);
    emit("success");
    emit("close");
  } catch (cause) {
    mapError(cause);
  } finally {
    submitting.value = false;
  }
}
</script>

<template>
  <DbModal
    :open="open"
    :title="title"
    size="sm"
    @close="!submitting && emit('close')">
    <form class="flex flex-col gap-4" novalidate @submit.prevent="submit">
      <DbInput
        v-model="name"
        :label="label"
        name="resource-name"
        placeholder="e.g. payment-service"
        mono
        :hint="hint"
        :error="error" />
      <div class="flex items-center justify-end gap-2">
        <DbButton variant="ghost" :disabled="submitting" @click="emit('close')">
          Cancel
        </DbButton>
        <DbButton variant="primary" type="submit" :loading="submitting">
          {{ submitLabel }}
        </DbButton>
      </div>
    </form>
  </DbModal>
</template>
