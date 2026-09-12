import { describe, expect, it } from "vitest";
import {
  isSafeRedirect,
  isValidEmail,
  isValidSecretKey,
  validatePasswordLength,
} from "./validation";

describe("shared validation", () => {
  it("accepts bounded email addresses and trims input", () => {
    expect(isValidEmail(" admin@example.com ")).toBe(true);
    expect(isValidEmail("admin@example")).toBe(false);
  });

  it("accepts only same-origin paths as redirect targets", () => {
    expect(isSafeRedirect("/projects")).toBe(true);
    expect(isSafeRedirect("/projects/p/acme/e/env_1")).toBe(true);
    expect(isSafeRedirect("https://evil.example.test/phish")).toBe(false);
    expect(isSafeRedirect("//evil.example.test/phish")).toBe(false);
    expect(isSafeRedirect("javascript:alert(1)")).toBe(false);
    expect(isSafeRedirect("workspace")).toBe(false);
  });

  it("counts password characters consistently with the server", () => {
    expect(validatePasswordLength("short")).toEqual({
      valid: false,
      code: "PASSWORD_TOO_SHORT",
    });
    expect(validatePasswordLength("😀".repeat(12))).toEqual({
      valid: true,
      code: undefined,
    });
    expect(validatePasswordLength("x".repeat(129))).toEqual({
      valid: false,
      code: "PASSWORD_TOO_LONG",
    });
  });

  it("matches the backend secret-key grammar", () => {
    expect(isValidSecretKey("DATABASE_URL")).toBe(true);
    expect(isValidSecretKey("DATABASE.URL")).toBe(false);
    expect(isValidSecretKey("1_DATABASE")).toBe(false);
    expect(isValidSecretKey("_DATABASE2")).toBe(true);
  });
});
