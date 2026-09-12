import { ref, watch, type Ref } from "vue";
import { useRoute, useRouter } from "vue-router";
import { parseEnvFile } from "~/utils/env-file";
import { useImportStore } from "~/stores/import.store";

/** Owns local .env parsing and navigation to the import review screen. */
export function useImportSecretsDialogController(
  environmentId: Ref<string>,
  open: Ref<boolean>,
) {
  const route = useRoute();
  const router = useRouter();
  const importStore = useImportStore();
  const parseErrors = ref<string[]>([]);
  const fileError = ref<string | null>(null);
  const parsing = ref(false);

  watch(open, (isOpen) => {
    if (isOpen) {
      parseErrors.value = [];
      fileError.value = null;
    }
  });

  async function processFile(file: File): Promise<boolean> {
    fileError.value = null;
    parseErrors.value = [];
    parsing.value = true;
    try {
      const parsed = parseEnvFile(await file.text());
      if (parsed.entries.length === 0 && parsed.errors.length > 0) {
        parseErrors.value = parsed.errors;
        return false;
      }
      importStore.begin({
        environmentId: environmentId.value,
        fileName: file.name,
        entries: parsed.entries,
        errors: parsed.errors,
      });
      await router.push({
        name: "environment-import",
        params: {
          projectRef: route.params.projectRef,
          environmentId: environmentId.value,
        },
      });
      return true;
    } catch {
      fileError.value = "The file could not be read.";
      return false;
    } finally {
      parsing.value = false;
    }
  }

  return { parseErrors, fileError, parsing, processFile };
}
