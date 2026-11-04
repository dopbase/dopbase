import { onBeforeUnmount, onMounted } from "vue";

/** Whether a keypress is the user typing, rather than a shortcut. */
function isTyping(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) return false;
  return (
    target.isContentEditable ||
    target.closest("input, textarea, select, [contenteditable]") !== null
  );
}

/**
 * Runs `handler` when `key` is pressed on its own. It pauses while focus is
 * in a field, while a modifier is held, and while a dialog is open, so it
 * never steals a letter someone meant to type.
 */
export function useSingleKeyShortcut(key: string, handler: () => void): void {
  function onKeydown(event: KeyboardEvent): void {
    if (event.key.toLowerCase() !== key.toLowerCase()) return;
    if (event.metaKey || event.ctrlKey || event.altKey || event.repeat) return;
    if (event.defaultPrevented || isTyping(event.target)) return;
    if (document.querySelector('[role="dialog"], [role="menu"]')) return;
    event.preventDefault();
    handler();
  }

  onMounted(() => document.addEventListener("keydown", onKeydown));
  onBeforeUnmount(() => document.removeEventListener("keydown", onKeydown));
}
