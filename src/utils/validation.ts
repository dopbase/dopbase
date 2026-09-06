/** Shared input rules used by account, login, user, and secret forms. */
export const MIN_PASSWORD_LENGTH = 12;
export const MAX_PASSWORD_LENGTH = 128;
export const MAX_SECRET_KEY_LENGTH = 128;

const EMAIL_PATTERN = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;
const SECRET_KEY_PATTERN = /^[A-Za-z_][A-Za-z0-9_]*$/;

export function isValidEmail(value: string): boolean {
  const email = value.trim();
  return email.length <= 254 && EMAIL_PATTERN.test(email);
}

export function validatePasswordLength(value: string):
  | { valid: true; code: undefined }
  | { valid: false; code: "PASSWORD_TOO_SHORT" | "PASSWORD_TOO_LONG" } {
  const length = Array.from(value).length;
  if (length < MIN_PASSWORD_LENGTH) {
    return { valid: false, code: "PASSWORD_TOO_SHORT" };
  }
  if (length > MAX_PASSWORD_LENGTH) {
    return { valid: false, code: "PASSWORD_TOO_LONG" };
  }
  return { valid: true, code: undefined };
}

export function isValidSecretKey(value: string): boolean {
  return (
    value.length > 0 &&
    value.length <= MAX_SECRET_KEY_LENGTH &&
    SECRET_KEY_PATTERN.test(value)
  );
}
