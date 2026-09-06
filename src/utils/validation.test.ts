import { describe, expect, it } from "vitest";
import {
  isValidEmail,
  isValidSecretKey,
  validatePasswordLength,
} from "./validation";

describe("shared validation", () => {
  it("accepts bounded email addresses and trims input", () => {
    expect(isValidEmail(" admin@example.com ")).toBe(true);
    expect(isValidEmail("admin@example")).toBe(false);
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
