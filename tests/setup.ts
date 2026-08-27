import { beforeEach } from "vitest";

/**
 * Installs a working, in-memory `localStorage` on the test global.
 *
 * Node ships an experimental Web Storage API whose `localStorage` is
 * file-backed. The test process starts without a valid `--localstorage-file`
 * path, so the built-in `localStorage` is a broken empty object whose
 * `getItem`/`setItem`/`removeItem` are not functions. It is exposed as a
 * configurable accessor on `globalThis` that the happy-dom environment cannot
 * replace, so we override it here with a real in-memory implementation. This
 * keeps tests (and code under test that reads/writes `localStorage`) working
 * without depending on Node CLI flags.
 */
function createLocalStorageMock(): Storage {
  const store = new Map<string, string>();

  return {
    getItem(key) {
      return store.has(key) ? (store.get(key) as string) : null;
    },
    setItem(key, value) {
      store.set(key, String(value));
    },
    removeItem(key) {
      store.delete(key);
    },
    clear() {
      store.clear();
    },
    key(index) {
      return index >= 0 && index < store.size
        ? (Array.from(store.keys())[index] as string)
        : null;
    },
    get length() {
      return store.size;
    },
  };
}

const localStorageMock = createLocalStorageMock();

for (const target of [globalThis, (globalThis as unknown as { window?: unknown }).window]) {
  if (target == null) continue;
  Object.defineProperty(target, "localStorage", {
    value: localStorageMock,
    configurable: true,
    writable: true,
  });
}

// Reset storage between tests so state never leaks across cases.
beforeEach(() => {
  localStorageMock.clear();
});
