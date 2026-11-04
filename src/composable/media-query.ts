import { onBeforeUnmount, ref, type Ref } from "vue";

/** Below Tailwind's `sm` breakpoint (40rem), where phones live. */
export const NARROW_SCREEN = "(max-width: 39.99rem)";

/**
 * Tracks a CSS media query, for markup that differs by more than styles
 * can say, such as a menu that becomes a bottom sheet on phones. False
 * where `matchMedia` doesn't exist.
 */
export function useMediaQuery(query: string): Ref<boolean> {
  const matches = ref(false);
  if (typeof window === "undefined" || !window.matchMedia) return matches;

  const list = window.matchMedia(query);
  matches.value = list.matches;
  const update = (event: MediaQueryListEvent) => {
    matches.value = event.matches;
  };
  list.addEventListener("change", update);
  onBeforeUnmount(() => list.removeEventListener("change", update));
  return matches;
}
