import { computed, onUnmounted, ref, watch, type Ref } from "vue";
import * as compareApi from "~/services/compare.api";
import type { CompareEnvironment, CompareResult } from "~/services/compare.api";
import * as secretsApi from "~/services/secrets.api";
import { reaches } from "~/services/access.api";
import { ApiError } from "~/services/http.client";
import { errorMessage } from "~/services/api-errors";
import type { Environment } from "~/services";

export const MAX_COMPARE = compareApi.MAX_COMPARE_ENVIRONMENTS;

/** What one cell says about a key, read against the base. */
export type CellStatus =
  | "base-set"
  | "base-missing"
  | "same"
  | "differs"
  | "missing"
  | "set"
  | "unknown";

export interface CompareColumn extends CompareEnvironment {
  canRead: boolean;
  canWrite: boolean;
}

export interface CompareCellView {
  column: CompareColumn;
  status: CellStatus;
  version: number | null;
  /** Not set here, and you can write here: clicking it fills the gap. */
  fillable: boolean;
}

export interface CompareRow {
  key: string;
  base: CompareCellView;
  cells: CompareCellView[];
  /** Missing somewhere, or different from the base. */
  different: boolean;
}

export type CompareView = "differences" | "all";

/** Filling one Not set cell: match another environment, or type a value. */
export interface FillDialog {
  key: string;
  target: CompareCellView;
  /** Cells that hold the key and can be read: what "Match" can copy. */
  sources: CompareCellView[];
  /** A source column id, or "own" to type a value. */
  choice: string;
  value: string;
}

const storageKey = (projectId: string) => `dopbase.compare.${projectId}`;

function readStored(projectId: string): string[] | null {
  try {
    const raw = localStorage.getItem(storageKey(projectId));
    const parsed: unknown = raw ? JSON.parse(raw) : null;
    return Array.isArray(parsed)
      ? parsed.filter((id): id is string => typeof id === "string")
      : null;
  } catch {
    return null;
  }
}

function writeStored(projectId: string, ids: string[]): void {
  try {
    localStorage.setItem(storageKey(projectId), JSON.stringify(ids));
  } catch {
    // Private windows can refuse storage; the choice just isn't remembered.
  }
}

function column(environment: CompareEnvironment): CompareColumn {
  return {
    ...environment,
    canRead: reaches(environment.level, "read"),
    canWrite: reaches(environment.level, "write"),
  };
}

/**
 * Compare panel controller. The environment open in the context bar is the
 * base; one other environment is read against it to start, and up to three
 * can be. The choice is remembered per project in this browser, and
 * switching base swaps the new base out of the list for the old one, so
 * the columns stay put.
 *
 * Values never reach the browser: the server says only whether each one
 * matches the base. A Not set cell can be filled by matching another
 * environment (copied on the server) or with a value you type.
 */
export function useComparePanelController(
  base: Ref<Environment>,
  environments: Ref<Environment[]>,
) {
  const projectId = computed(() => base.value.projectId);
  /** Every other environment, in the project's order. */
  const others = computed(() =>
    environments.value.filter((candidate) => candidate.id !== base.value.id),
  );

  function initial(): string[] {
    const known = new Set(others.value.map((candidate) => candidate.id));
    const stored = readStored(projectId.value);
    const ids = stored
      ? stored.filter((id) => known.has(id))
      : others.value.slice(0, 1).map((candidate) => candidate.id);
    return ids.slice(0, MAX_COMPARE);
  }

  const selectedIds = ref<string[]>(initial());

  function select(ids: string[]): void {
    selectedIds.value = ids.slice(0, MAX_COMPARE);
    writeStored(projectId.value, selectedIds.value);
  }

  // Switching base: the new base leaves the columns and the old one takes
  // its place, so prod ⇄ staging keeps the same environments in view. A
  // project change starts from that project's remembered columns instead.
  watch(
    () => [base.value.id, base.value.projectId] as const,
    ([current, project], [previous, previousProject]) => {
      if (project !== previousProject) {
        selectedIds.value = initial();
        return;
      }
      const ids = selectedIds.value;
      const at = ids.indexOf(current);
      if (at === -1) return;
      const next = [...ids];
      if (!ids.includes(previous)) next[at] = previous;
      else next.splice(at, 1);
      select(next);
    },
  );
  // Environments that disappear leave the list.
  watch(
    () => others.value.map((candidate) => candidate.id).join(","),
    () => {
      const known = new Set(others.value.map((candidate) => candidate.id));
      const kept = selectedIds.value.filter((id) => known.has(id));
      if (kept.length !== selectedIds.value.length) selectedIds.value = kept;
    },
  );

  const selected = computed(() =>
    selectedIds.value
      .map((id) => others.value.find((candidate) => candidate.id === id))
      .filter((candidate): candidate is Environment => Boolean(candidate)),
  );
  const addable = computed(() =>
    others.value.filter(
      (candidate) => !selectedIds.value.includes(candidate.id),
    ),
  );
  /** Add environment shows until three are compared. */
  const canAdd = computed(
    () => selectedIds.value.length < MAX_COMPARE && addable.value.length > 0,
  );

  function add(id: string): void {
    if (!canAdd.value || selectedIds.value.includes(id)) return;
    select([...selectedIds.value, id]);
  }

  function remove(id: string): void {
    select(selectedIds.value.filter((candidate) => candidate !== id));
  }

  // ---------------------------------------------------------------------
  // Loading
  // ---------------------------------------------------------------------

  /** "FEATURE_FLAGS set in prod.", shown after a fill. */
  const filled = ref<string | null>(null);
  const result = ref<CompareResult | null>(null);
  const loading = ref(false);
  const loadError = ref<string | null>(null);
  let request: AbortController | null = null;

  async function load(): Promise<void> {
    request?.abort();
    const current = new AbortController();
    request = current;
    loading.value = true;
    loadError.value = null;
    try {
      const next = await compareApi.compareEnvironments(
        base.value.id,
        selected.value.map((candidate) => candidate.id),
        current.signal,
      );
      if (!current.signal.aborted) result.value = next;
    } catch (cause) {
      if (current.signal.aborted) return;
      loadError.value =
        errorMessage(cause, "Could not compare these environments.") || null;
    } finally {
      if (request === current) loading.value = false;
    }
  }

  watch(
    () => base.value.id,
    () => {
      // Another base: the old table would read against the wrong one, and
      // the last fill's note belongs to it.
      result.value = null;
      filled.value = null;
    },
  );
  watch(
    () => [base.value.id, selectedIds.value.join(",")],
    () => void load(),
    { immediate: true },
  );
  onUnmounted(() => request?.abort());

  // ---------------------------------------------------------------------
  // Rows
  // ---------------------------------------------------------------------

  const baseColumn = computed(() =>
    result.value ? column(result.value.base) : null,
  );
  const columns = computed(() =>
    result.value ? result.value.environments.map(column) : [],
  );
  /** Columns that show only which keys exist. */
  const namesOnly = computed(() =>
    baseColumn.value?.valuesCompared
      ? columns.value.filter((candidate) => !candidate.valuesCompared)
      : [],
  );

  const rows = computed<CompareRow[]>(() => {
    const data = result.value;
    const baseCol = baseColumn.value;
    if (!data || !baseCol) return [];
    return data.keys.map((entry) => {
      const baseCell: CompareCellView = {
        column: baseCol,
        status: entry.base === null ? "base-missing" : "base-set",
        version: entry.base,
        fillable: entry.base === null && baseCol.canWrite,
      };
      const cells = entry.others.map<CompareCellView>((cell, index) => {
        const col = columns.value[index]!;
        let status: CellStatus;
        if (cell.version === null) status = "missing";
        else if (entry.base === null) status = "set";
        else if (cell.sameAsBase === null) status = "unknown";
        else status = cell.sameAsBase ? "same" : "differs";
        return {
          column: col,
          status,
          version: cell.version,
          fillable: cell.version === null && col.canWrite,
        };
      });
      const different =
        entry.base === null ||
        cells.some(
          (cell) => cell.status === "missing" || cell.status === "differs",
        );
      return { key: entry.key, base: baseCell, cells, different };
    });
  });

  const view = ref<CompareView>("differences");
  const filter = ref("");
  const differences = computed(() => rows.value.filter((row) => row.different));
  const visibleRows = computed(() => {
    const needle = filter.value.trim().toLowerCase();
    const list = view.value === "differences" ? differences.value : rows.value;
    return needle
      ? list.filter((row) => row.key.toLowerCase().includes(needle))
      : list;
  });

  const summary = computed(() => ({
    missingInBase: rows.value.filter((row) => row.base.version === null).length,
    missingElsewhere: rows.value.filter((row) =>
      row.cells.some((cell) => cell.status === "missing"),
    ).length,
    differs: rows.value.filter((row) =>
      row.cells.some((cell) => cell.status === "differs"),
    ).length,
    same: rows.value.length - differences.value.length,
  }));

  // ---------------------------------------------------------------------
  // Filling a Not set cell
  // ---------------------------------------------------------------------

  const fillDialog = ref<FillDialog | null>(null);
  const filling = ref(false);
  const fillError = ref<string | null>(null);

  function openFill(row: CompareRow, target: CompareCellView): void {
    if (!target.fillable) return;
    fillError.value = null;
    filled.value = null;
    // The base for any other column; for the base, whichever columns have it.
    const sources = (
      target.column.id === row.base.column.id ? row.cells : [row.base]
    ).filter((cell) => cell.version !== null && cell.column.canRead);
    fillDialog.value = {
      key: row.key,
      target,
      sources,
      choice: sources[0]?.column.id ?? "own",
      value: "",
    };
  }

  function cancelFill(): void {
    if (!filling.value) fillDialog.value = null;
  }

  async function confirmFill(): Promise<void> {
    const dialog = fillDialog.value;
    if (!dialog) return;
    const own = dialog.choice === "own";
    if (own && dialog.value === "") {
      fillError.value = "Enter a value, or match another environment.";
      return;
    }
    filling.value = true;
    fillError.value = null;
    try {
      if (own) {
        await secretsApi.setSecret(
          dialog.target.column.id,
          dialog.key,
          dialog.value,
        );
      } else {
        await compareApi.copySecret(
          dialog.target.column.id,
          dialog.key,
          dialog.choice,
          dialog.target.version,
        );
      }
      fillDialog.value = null;
      filled.value = `${dialog.key} set in ${dialog.target.column.name}.`;
      void load();
    } catch (cause) {
      fillError.value =
        cause instanceof ApiError && cause.status === 409
          ? "It was set since this page loaded. Close this to see the latest."
          : errorMessage(cause, "The secret could not be set.");
    } finally {
      filling.value = false;
    }
  }

  return {
    others,
    selected,
    addable,
    canAdd,
    add,
    remove,
    result,
    loading,
    loadError,
    load,
    baseColumn,
    columns,
    namesOnly,
    rows,
    view,
    filter,
    differences,
    visibleRows,
    summary,
    fillDialog,
    filling,
    fillError,
    filled,
    openFill,
    cancelFill,
    confirmFill,
  };
}

export type ComparePanelController = ReturnType<
  typeof useComparePanelController
>;
