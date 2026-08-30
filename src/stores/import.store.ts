import { ref } from "vue";
import { defineStore } from "pinia";
import type { ParsedEnvEntry } from "~/utils/env-file";

export interface PendingImport {
  environmentId: string;
  fileName: string;
  entries: ParsedEnvEntry[];
  /** Human-readable problems for lines skipped during parsing. */
  errors: string[];
}

/**
 * Carries a parsed `.env` file from the upload dialog to the full-page
 * import review (`environment-import` route). In-memory only: reloading
 * the review route loses the parsed file and the page bounces back to the
 * environment, so values never persist outside component memory.
 */
export const useImportStore = defineStore("import", () => {
  const pending = ref<PendingImport | null>(null);

  function begin(payload: PendingImport): void {
    pending.value = payload;
  }

  /** Drops the pending import (after apply, cancel, or leaving the page). */
  function clear(): void {
    pending.value = null;
  }

  return { pending, begin, clear };
});
