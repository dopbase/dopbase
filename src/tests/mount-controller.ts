import { createApp } from "vue";
import { onTestFinished } from "vitest";

/** Run a controller in component setup and dispose it when the test ends. */
export function mountController<T>(factory: () => T) {
  let controller!: T;
  const app = createApp({
    setup() {
      controller = factory();
      return () => null;
    },
  });
  app.mount(document.createElement("div"));

  let mounted = true;
  function unmount(): void {
    if (!mounted) return;
    mounted = false;
    app.unmount();
  }

  onTestFinished(unmount);
  return { controller, unmount };
}
