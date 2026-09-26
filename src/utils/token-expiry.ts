export const MAX_TOKEN_EXPIRY_HOURS = 26280;
export const MAX_TOKEN_EXPIRY_DAYS = 1095;

export const tokenExpiryOptions = [
  { value: "never", label: "No expiry" },
  { value: "1h", label: "1 hour" },
  { value: "1d", label: "1 day" },
  { value: "3d", label: "3 days" },
  { value: "7d", label: "7 days" },
  { value: "30d", label: "30 days" },
  { value: "custom", label: "Custom" },
];

const presetExpiries = new Set(
  tokenExpiryOptions
    .filter((option) => option.value !== "custom")
    .map((option) => option.value),
);

export const tokenExpiryUnits = [
  { value: "h", label: "Hours" },
  { value: "d", label: "Days" },
];

export function tokenExpiresIn(
  choice: string,
  amount: string,
  unit: string,
): string {
  if (choice !== "custom") {
    if (!presetExpiries.has(choice)) throw new Error("Choose a valid expiry.");
    return choice;
  }
  const max =
    unit === "h"
      ? MAX_TOKEN_EXPIRY_HOURS
      : unit === "d"
        ? MAX_TOKEN_EXPIRY_DAYS
        : 0;
  if (!/^[0-9]+$/.test(amount) || !max) {
    throw new Error("Enter a whole number of hours or days.");
  }
  const value = Number(amount);
  if (!Number.isSafeInteger(value) || value < 1 || value > max) {
    throw new Error(
      `Enter a value from 1 to ${max} ${unit === "h" ? "hours" : "days"}.`,
    );
  }
  return `${value}${unit}`;
}
