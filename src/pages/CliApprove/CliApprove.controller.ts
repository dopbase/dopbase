import { computed, onBeforeUnmount, onMounted, ref } from "vue";
import { useRoute, useRouter } from "vue-router";
import {
  approveCliLogin,
  denyCliLogin,
  inspectCliLogin,
} from "~/services/auth.api";
import type { CliLoginRequest } from "~/services/auth.api";
import { ApiError } from "~/services/http.client";
import { useAuthStore } from "~/stores/auth.store";
import {
  requestConfirmation,
  useReauthentication,
} from "~/composable/reauthentication";
import { formatRole } from "~/utils/format";

export type CliApproveStep =
  "checking" | "enter" | "review" | "approved" | "denied" | "expired";

/** The characters a code is made of; see the server's user code alphabet. */
export const CODE_ALPHABET = "BCDFGHJKLMNPQRSTVWXZ";
export const CODE_LENGTH = 8;
/** Seconds the approved page waits before it closes its tab. */
export const CLOSE_DELAY_SECONDS = 5;
/** The CLI session length when an older server doesn't say. */
export const DEFAULT_SESSION_DAYS = 90;

/** `4:32` for 272 seconds. */
export function formatCountdown(seconds: number): string {
  const safe = Math.max(0, Math.floor(seconds));
  return `${Math.floor(safe / 60)}:${String(safe % 60).padStart(2, "0")}`;
}

/** `wdjb-mjht`, `WDJB MJHT`, and `WDJBMJHT` are the same code. */
export function normalizeCode(value: unknown): string {
  if (typeof value !== "string") return "";
  return [...value.toUpperCase()]
    .filter((character) => CODE_ALPHABET.includes(character))
    .join("")
    .slice(0, CODE_LENGTH);
}

export function displayCode(code: string): string {
  return code.length > 4 ? `${code.slice(0, 4)}-${code.slice(4)}` : code;
}

/**
 * CLI approval: the page `dopbase login --browser` links to.
 *
 * It uses whatever session this browser already has and says plainly which
 * account will be signed in, with a way to switch. The link usually opens in
 * a new tab, so the page fetches the session first, which also gives the
 * tab its CSRF token. Approving needs a recent confirmation, as any other
 * sensitive action does; the confirm dialog asks for it without signing out.
 */
export function useCliApproveController() {
  const route = useRoute();
  const router = useRouter();
  const auth = useAuthStore();
  const { runWithReauth } = useReauthentication();

  const code = ref(normalizeCode(route.query.code));
  const step = ref<CliApproveStep>("checking");
  const request = ref<CliLoginRequest | null>(null);
  const codeError = ref<string | null>(null);
  const formError = ref<string | null>(null);
  const looking = ref(false);
  const deciding = ref<"approve" | "deny" | null>(null);
  const closingIn = ref<number | null>(null);
  const closeBlocked = ref(false);
  let countdown: ReturnType<typeof setInterval> | null = null;
  let closeCheck: ReturnType<typeof setTimeout> | null = null;
  /** Seconds until the request expires, while it's on screen. */
  const secondsLeft = ref<number | null>(null);
  let expiryTimer: ReturnType<typeof setInterval> | null = null;

  const isConfirmation = computed(() => request.value?.purpose === "reauth");
  /** The machine's name, for the button and the done screen. */
  const machine = computed(() => request.value?.hostname ?? null);
  const sessionDays = computed(
    () => request.value?.sessionDays ?? DEFAULT_SESSION_DAYS,
  );
  const expiresIn = computed(() =>
    secondsLeft.value === null ? "" : formatCountdown(secondsLeft.value),
  );

  function stopExpiry(): void {
    if (expiryTimer) clearInterval(expiryTimer);
    expiryTimer = null;
  }

  /**
   * Count down to the request's expiry; at zero it can't be approved. The
   * server just said it is pending, so a deadline already past, or further
   * off than the request's whole lifetime, means this clock is off: count
   * the lifetime from now instead.
   */
  function watchExpiry(): void {
    stopExpiry();
    const expiresAt = request.value ? Date.parse(request.value.expiresAt) : NaN;
    const createdAt = request.value ? Date.parse(request.value.createdAt) : NaN;
    if (Number.isNaN(expiresAt) || Number.isNaN(createdAt)) {
      secondsLeft.value = null;
      return;
    }
    const lifetime = expiresAt - createdAt;
    const remaining = expiresAt - Date.now();
    const deadline =
      Date.now() +
      (remaining > 0 && remaining <= lifetime ? remaining : lifetime);
    const tick = (): void => {
      const left = Math.round((deadline - Date.now()) / 1000);
      secondsLeft.value = Math.max(0, left);
      if (left <= 0) {
        stopExpiry();
        if (step.value === "review") step.value = "expired";
      }
    };
    tick();
    expiryTimer = setInterval(tick, 1000);
  }
  /** The account an approval signs in, shown on every step. */
  const account = computed(() => {
    const session = auth.session;
    if (!session) return null;
    return {
      email: session.email,
      role: formatRole(session.role),
      initial: session.email.charAt(0).toUpperCase(),
    };
  });

  function describe(error: unknown, fallback: string): string {
    if (error instanceof ApiError) {
      if (error.hasCode("RATE_LIMITED")) {
        return "Too many wrong codes. Wait a few minutes, then try again.";
      }
      if (error.hasCode("DEVICE_CODE_INVALID")) {
        return "That code is wrong or has expired. Check your terminal, or run dopbase login again.";
      }
    }
    return fallback;
  }

  async function goToSignIn(): Promise<void> {
    // Forget the session here too, quietly: otherwise the login page sees
    // a signed-in store, sends people to the dashboard, and the dashboard
    // shows "session expired".
    auth.clearLocalSession();
    await router.replace({
      name: "login",
      query: {
        redirect: `/cli/approve?code=${displayCode(code.value)}`,
        notice: "cli-approve",
      },
    });
  }

  /** Sign out and sign in as someone else, then come back to the same code. */
  async function switchAccount(): Promise<void> {
    try {
      await auth.logout();
    } catch {
      // Already signed out: the next sign-in replaces the cookie anyway.
    }
    await goToSignIn();
  }

  /**
   * Handle the answers that mean "sign in first": no session at all, or a
   * session this tab still cannot write with, which a new sign-in fixes.
   */
  async function recover(error: unknown): Promise<boolean> {
    if (!(error instanceof ApiError)) return false;
    if (error.status === 401) {
      await goToSignIn();
      return true;
    }
    if (error.hasCode("AUTHORIZATION_DENIED")) {
      await switchAccount();
      return true;
    }
    return false;
  }

  async function start(): Promise<void> {
    try {
      // Quiet: no session here just means signing in, not "expired".
      await auth.fetchSession({ quiet: true });
    } catch (error) {
      if (await recover(error)) return;
      step.value = "enter";
      codeError.value =
        "Could not reach the server. Reload the page to try again.";
      return;
    }
    if (code.value.length === CODE_LENGTH) await lookUp();
    else step.value = "enter";
  }

  async function lookUp(): Promise<void> {
    codeError.value = null;
    formError.value = null;
    if (code.value.length !== CODE_LENGTH) {
      codeError.value = "Enter all 8 characters shown in your terminal.";
      return;
    }
    looking.value = true;
    try {
      request.value = await inspectCliLogin(code.value);
      step.value = "review";
      watchExpiry();
    } catch (error) {
      if (await recover(error)) return;
      step.value = "enter";
      codeError.value = describe(error, "Could not look up this code.");
    } finally {
      looking.value = false;
    }
  }

  function stopClosing(): void {
    if (countdown) clearInterval(countdown);
    if (closeCheck) clearTimeout(closeCheck);
    countdown = null;
    closeCheck = null;
    closingIn.value = null;
  }

  /**
   * Count down, then close the tab. Browsers only let a page close a tab a
   * script opened or one with a single history entry, and this tab usually
   * went through sign-in first, so it may stay open. Say so when it does.
   */
  function startClosing(): void {
    stopClosing();
    closeBlocked.value = false;
    closingIn.value = CLOSE_DELAY_SECONDS;
    countdown = setInterval(() => {
      const left = (closingIn.value ?? 1) - 1;
      if (left > 0) {
        closingIn.value = left;
        return;
      }
      stopClosing();
      window.close();
      // A tab that did close never gets here.
      closeCheck = setTimeout(() => {
        closeBlocked.value = true;
      }, 300);
    }, 1000);
  }

  async function decide(approve: boolean): Promise<void> {
    formError.value = null;
    deciding.value = approve ? "approve" : "deny";
    try {
      if (approve) {
        await runWithReauth(
          () => approveCliLogin(code.value),
          undefined,
          () =>
            requestConfirmation(
              isConfirmation.value
                ? "Confirm it's you before this terminal can show secrets."
                : `Confirm it's you before signing in this terminal. It stays signed in for up to ${sessionDays.value} days.`,
            ),
        );
      } else {
        await denyCliLogin(code.value);
      }
      stopExpiry();
      step.value = approve ? "approved" : "denied";
      if (approve) startClosing();
    } catch (error) {
      // Closing the confirm dialog leaves the request for another try.
      if (error instanceof DOMException && error.name === "AbortError") return;
      if (await recover(error)) return;
      formError.value = describe(
        error,
        approve ? "Could not approve this request." : "Could not deny it.",
      );
    } finally {
      deciding.value = null;
    }
  }

  function startOver(): void {
    stopClosing();
    stopExpiry();
    secondsLeft.value = null;
    closeBlocked.value = false;
    step.value = "enter";
    request.value = null;
    code.value = "";
    codeError.value = null;
    formError.value = null;
  }

  onMounted(() => void start());
  onBeforeUnmount(() => {
    stopClosing();
    stopExpiry();
  });

  return {
    step,
    code,
    request,
    account,
    isConfirmation,
    machine,
    sessionDays,
    secondsLeft,
    expiresIn,
    codeError,
    formError,
    looking,
    deciding,
    closingIn,
    closeBlocked,
    keepOpen: stopClosing,
    lookUp,
    approve: () => decide(true),
    deny: () => decide(false),
    switchAccount,
    startOver,
  };
}
