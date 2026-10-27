/** Mobile project rail uses z-60; modals must sit above it. */
const BASE_Z = 70;
const PRIORITY_Z = 100;

let openCount = 0;

export function acquireModalLayer(priority = false): number {
  openCount += 1;
  if (priority) {
    return PRIORITY_Z + openCount;
  }
  return BASE_Z + openCount * 10;
}

export function releaseModalLayer(): void {
  openCount = Math.max(0, openCount - 1);
}
