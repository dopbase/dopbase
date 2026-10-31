/**
 * The recovery kit setup asks root to save: the master key as hex, the
 * server's address, and how to restore a backup with it.
 */

/** The master key as 64 lowercase hex characters. */
export function bytesToHex(bytes: Uint8Array): string {
  return Array.from(bytes, (byte) => byte.toString(16).padStart(2, "0")).join(
    "",
  );
}

/** `4a2f8b…` in groups of eight, so it can be read back and checked. */
export function groupKey(hex: string): string {
  return hex.match(/.{1,8}/g)?.join("-") ?? "";
}

/** The same shape with every character hidden. */
export function maskKey(hex: string): string {
  return groupKey(hex).replace(/[^-]/g, "•");
}

export interface RecoveryKitDetails {
  /** `secrets.acme.dev` */
  host: string;
  /** `https://secrets.acme.dev` */
  origin: string;
  /** Root's email. */
  email: string;
  key: string;
  createdAt: Date;
}

/** The text file "Download recovery kit" saves. */
export function recoveryKitText(kit: RecoveryKitDetails): string {
  return [
    "Dopbase recovery kit",
    "",
    `Server: ${kit.origin}`,
    `Root account: ${kit.email}`,
    `Saved: ${kit.createdAt.toISOString()}`,
    "",
    "Master key (hex):",
    kit.key,
    "",
    "Every secret on this server, and every backup of it, is encrypted with",
    "this key. Keep this file somewhere other than the server: a password",
    "manager or a printed copy.",
    "",
    "To restore a backup on a new server:",
    "1. Start Dopbase there and open the setup link it prints.",
    "2. Choose Restore from backup and pick the .dop file.",
    "3. Paste the master key above.",
    "",
    "Root can download the key again from Status while this server runs.",
    "",
  ].join("\n");
}

/** File name for the kit: `dopbase-recovery-kit-secrets.acme.dev.txt`. */
export function recoveryKitFileName(host: string): string {
  const safe = host.replace(/[^a-z0-9.-]/gi, "-") || "server";
  return `dopbase-recovery-kit-${safe}.txt`;
}

export type PasswordStrength = "empty" | "short" | "weak" | "fair" | "strong";

/**
 * A rough read on a new password for the meter under the field: length
 * counts most, mixing kinds of characters helps a little. The server only
 * enforces the length limits.
 */
export function passwordStrength(
  password: string,
  minLength: number,
): PasswordStrength {
  if (password.length === 0) return "empty";
  if (password.length < minLength) return "short";
  const kinds = [/[a-z]/, /[A-Z]/, /[0-9]/, /[^a-zA-Z0-9]/].filter((kind) =>
    kind.test(password),
  ).length;
  if (password.length >= 16 || (password.length >= 14 && kinds >= 3)) {
    return "strong";
  }
  if (password.length >= 14 || kinds >= 3) return "fair";
  return "weak";
}

export const STRENGTH_LABELS: Record<PasswordStrength, string> = {
  empty: "",
  short: "Too short",
  weak: "Weak",
  fair: "Fair",
  strong: "Strong",
};
