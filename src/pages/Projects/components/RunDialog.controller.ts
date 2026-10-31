import { computed, onUnmounted, ref, watch, type Ref } from "vue";
import * as tokensApi from "~/services/tokens.api";
import type { RunnerToken } from "~/services/tokens.api";
import type { Environment } from "~/services";
import { formatDate, formatRelativeTime } from "~/utils/format";

export type RunTarget = "cli" | "docker" | "github";

export const RUN_TARGETS: readonly { value: RunTarget; label: string }[] = [
  { value: "cli", label: "CLI" },
  { value: "docker", label: "Docker" },
  { value: "github", label: "GitHub Actions" },
];

/** A runner token still in use: not revoked and not past its expiry. */
export function isActiveToken(token: RunnerToken, now = Date.now()): boolean {
  if (token.revokedAt) return false;
  return token.expiresAt === null || new Date(token.expiresAt).getTime() > now;
}

/**
 * Run dialog controller: how to run an app with this environment's values,
 * for the CLI, Docker or GitHub Actions, and which runner token deploys use.
 * People run by `project/environment`; machines use the immutable
 * environment id with a runner token, so a rename never breaks a deploy.
 *
 * Tokens load when the dialog opens, and only for viewers who manage the
 * environment, because the server refuses the list to everyone else.
 */
export function useRunDialogController(
  projectName: Ref<string>,
  environment: Ref<Environment>,
  canManage: Ref<boolean>,
  open: Ref<boolean>,
) {
  const target = ref<RunTarget>("cli");
  const tokens = ref<RunnerToken[] | null>(null);
  let request: AbortController | null = null;

  const serverUrl = window.location.origin;
  const reference = computed(
    () => `${projectName.value}/${environment.value.name}`,
  );

  const command = computed(() => {
    const env = environment.value;
    switch (target.value) {
      case "docker":
        return [
          "docker run \\",
          `  -e DOPBASE_URL=${serverUrl} \\`,
          "  -e DOPBASE_TOKEN \\",
          `  your-image dopbase run ${env.id} -- npm start`,
        ].join("\n");
      case "github":
        return [
          "- name: Run with Dopbase",
          `  run: dopbase run ${env.id} -- npm test`,
          "  env:",
          `    DOPBASE_URL: ${serverUrl}`,
          "    DOPBASE_TOKEN: ${{ secrets.DOPBASE_TOKEN }}",
        ].join("\n");
      default:
        return [
          "dopbase login",
          `dopbase run ${reference.value} -- npm start`,
        ].join("\n");
    }
  });

  const hint = computed(() => {
    switch (target.value) {
      case "docker":
        return "The image needs the dopbase CLI. Pass a runner token for this environment as DOPBASE_TOKEN; the id never changes, even if the environment is renamed.";
      case "github":
        return "Save a runner token for this environment as the DOPBASE_TOKEN repository secret. The id never changes, even if the environment is renamed.";
      default:
        return "Values load into the process when it starts. Nothing is written to disk, and each read is recorded in the audit log.";
    }
  });

  /** The CLI tip: set the default once, then run without naming it. */
  const defaultCommand = computed(
    () => `dopbase env default ${reference.value}`,
  );

  async function loadTokens(): Promise<void> {
    request?.abort();
    tokens.value = null;
    if (!open.value || !canManage.value) return;
    const current = new AbortController();
    request = current;
    const id = environment.value.id;
    try {
      const result = await tokensApi.listTokens(id, current.signal);
      if (!current.signal.aborted && environment.value.id === id) {
        tokens.value = result;
      }
    } catch {
      // The dialog still shows the commands without the token summary.
    }
  }

  watch([open, () => environment.value.id, canManage], loadTokens, {
    immediate: true,
  });
  onUnmounted(() => request?.abort());

  /** The active token used most recently, to show as the deploy token. */
  const deployToken = computed(() => {
    const now = Date.now();
    const active = (tokens.value ?? []).filter((token) =>
      isActiveToken(token, now),
    );
    active.sort((a, b) =>
      (b.lastUsedAt ?? b.createdAt).localeCompare(a.lastUsedAt ?? a.createdAt),
    );
    return active[0] ?? null;
  });

  const deployTokenSummary = computed(() => {
    const token = deployToken.value;
    if (!token) return null;
    const read = token.lastUsedAt
      ? `Last read ${formatRelativeTime(token.lastUsedAt)}.`
      : "Not used yet.";
    const expires = token.expiresAt
      ? `Expires ${formatDate(token.expiresAt)}.`
      : "Never expires.";
    return `${read} ${expires}`;
  });

  return {
    target,
    command,
    hint,
    defaultCommand,
    tokensLoaded: computed(() => tokens.value !== null),
    deployToken,
    deployTokenSummary,
  };
}
