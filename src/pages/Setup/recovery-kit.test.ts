import { describe, expect, it } from "vitest";
import {
  bytesToHex,
  groupKey,
  maskKey,
  passwordStrength,
  recoveryKitFileName,
  recoveryKitText,
} from "./recovery-kit";

describe("recovery kit", () => {
  it("turns the 32-byte key into 64 hex characters", () => {
    const bytes = Uint8Array.from({ length: 32 }, (_, index) => index);
    const hex = bytesToHex(bytes);
    expect(hex).toHaveLength(64);
    expect(hex.startsWith("000102030405")).toBe(true);
  });

  it("groups the key in eights and hides it in the same shape", () => {
    const hex = "0123456789abcdef".repeat(4);
    expect(groupKey(hex)).toBe(
      "01234567-89abcdef-01234567-89abcdef-01234567-89abcdef-01234567-89abcdef",
    );
    expect(maskKey(hex)).toBe(groupKey(hex).replace(/[0-9a-f]/g, "•"));
  });

  it("puts the key, the server and the restore steps in the kit", () => {
    const text = recoveryKitText({
      host: "secrets.dopbase.com",
      origin: "https://secrets.dopbase.com",
      email: "root@dopbase.com",
      key: "ab".repeat(32),
      createdAt: new Date("2026-10-31T09:00:00Z"),
    });
    expect(text).toContain("Server: https://secrets.dopbase.com");
    expect(text).toContain("Root account: root@dopbase.com");
    expect(text).toContain("ab".repeat(32));
    expect(text).toContain("Choose Restore a backup");
    expect(text).toContain("2026-10-31T09:00:00.000Z");
  });

  it("names the file after the server, without path characters", () => {
    expect(recoveryKitFileName("secrets.dopbase.com:8443")).toBe(
      "dopbase-recovery-kit-secrets.dopbase.com-8443.txt",
    );
    expect(recoveryKitFileName("")).toBe("dopbase-recovery-kit-server.txt");
  });
});

describe("passwordStrength", () => {
  it("rates length first", () => {
    expect(passwordStrength("", 12)).toBe("empty");
    expect(passwordStrength("short1!", 12)).toBe("short");
    expect(passwordStrength("abcdefghijkl", 12)).toBe("weak");
    expect(passwordStrength("abcdefghijklmn", 12)).toBe("fair");
    expect(passwordStrength("correct horse battery", 12)).toBe("strong");
  });

  it("gives mixed characters credit before a passphrase's length", () => {
    expect(passwordStrength("Abcdefgh1234", 12)).toBe("fair");
    expect(passwordStrength("Abcdefgh1234!x", 12)).toBe("strong");
  });
});
