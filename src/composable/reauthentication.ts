import { ref } from "vue";
import { onReauthenticationRequired } from "~/services/http.client";
import { useAuthStore } from "~/stores/auth.store";

const isOpen = ref(false);
const error = ref<string | null>(null);
const submitting = ref(false);

interface PendingOperation {
  action: () => Promise<void>;
  resolve: () => void;
  reject: (cause: unknown) => void;
  signal?: AbortSignal;
  abort?: () => void;
}

const pending: PendingOperation[] = [];

// One listener serves every caller; mounting a panel no longer grows the
// HTTP client's global listener set.
onReauthenticationRequired(() => {
  error.value = null;
  isOpen.value = true;
});

function isChallenge(cause: unknown): boolean {
  return (
    cause instanceof Object &&
    "status" in cause &&
    (cause as { status: number }).status === 403 &&
    "hasCode" in cause &&
    (cause as { hasCode: (code: string) => boolean }).hasCode(
      "RECENT_AUTHENTICATION_REQUIRED",
    )
  );
}

function cancellation(): DOMException {
  return new DOMException(
    "The reauthentication operation was cancelled.",
    "AbortError",
  );
}

export function useReauthentication() {
  const auth = useAuthStore();

  async function submit(password: string): Promise<boolean> {
    submitting.value = true;
    error.value = null;
    try {
      await auth.reauthenticate(password);
    } catch {
      error.value = "The password is incorrect.";
      submitting.value = false;
      return false;
    }

    isOpen.value = false;
    submitting.value = false;
    const operations = pending.splice(0);
    await Promise.allSettled(
      operations.map(async (operation) => {
        if (operation.abort) {
          operation.signal?.removeEventListener("abort", operation.abort);
        }
        if (operation.signal?.aborted) {
          operation.reject(cancellation());
          return;
        }
        try {
          await operation.action();
          operation.resolve();
        } catch (cause) {
          operation.reject(cause);
        }
      }),
    );
    return true;
  }

  function dismiss(): void {
    if (submitting.value) return;
    isOpen.value = false;
    error.value = null;
    for (const operation of pending.splice(0)) {
      if (operation.abort) {
        operation.signal?.removeEventListener("abort", operation.abort);
      }
      operation.reject(cancellation());
    }
  }

  /** Resolves only when the operation completes, including a parked retry. */
  async function runWithReauth(
    action: () => Promise<void>,
    signal?: AbortSignal,
    onPark?: () => void,
  ): Promise<void> {
    if (signal?.aborted) throw cancellation();
    try {
      await action();
    } catch (cause) {
      if (!isChallenge(cause)) throw cause;
      onPark?.();
      await new Promise<void>((resolve, reject) => {
        const operation: PendingOperation = { action, resolve, reject, signal };
        if (signal) {
          operation.abort = () => {
            const index = pending.indexOf(operation);
            if (index >= 0) pending.splice(index, 1);
            reject(cancellation());
          };
          signal.addEventListener("abort", operation.abort, { once: true });
        }
        pending.push(operation);
      });
    }
  }

  return { isOpen, error, submitting, submit, dismiss, runWithReauth };
}
