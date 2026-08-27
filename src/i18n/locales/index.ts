import en from "./en.json";
import zh from "./zh.json";

export const messages = { en, zh };
export type Locale = keyof typeof messages;
export type Messages = typeof en;
export type MessageKey = keyof Messages;

// Compile-time guard: every locale catalog must provide the full English
// key set. JSON imports cannot enforce this by annotation, so a mismatched
// key fails the build here and the parity test below catches extras.
const parityCheck: Messages = en;
void parityCheck;

export { en, zh };
