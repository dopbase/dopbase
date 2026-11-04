/**
 * AuthArtwork controller: which artwork the sign-in aside shows.
 *
 * One is picked at random each time the aside mounts, never the same as
 * the one shown last, so a returning visitor always sees a new one. The
 * last pick lives in `localStorage`; when storage is refused every
 * artwork stays in the draw.
 */

export const ARTWORKS = [
  { name: "vault", caption: "Encrypted before it's stored." },
  { name: "masked", caption: "Hidden until you choose to reveal." },
  { name: "dial", caption: "Opened only by the people you allow." },
  { name: "layers", caption: "Prod stays apart from dev." },
  { name: "network", caption: "Every app reads from one place." },
] as const;

export type Artwork = (typeof ARTWORKS)[number];
export type ArtworkName = Artwork["name"];

const STORAGE_KEY = "dopbase.auth-artwork";

/** Artworks are drawn on an 880×900 frame; this scales a frame pixel. */
export const ART_WIDTH = 880;
export const artPx = (px: number) => `calc(${px} * 100cqw / ${ART_WIDTH})`;

/** Any artwork but `last`. `random` returns a number in [0, 1). */
export function pickArtwork(
  last: string | null,
  random: () => number = Math.random,
): Artwork {
  const choices = ARTWORKS.filter((artwork) => artwork.name !== last);
  return choices[Math.floor(random() * choices.length)] ?? ARTWORKS[0];
}

function readLast(): string | null {
  try {
    return localStorage.getItem(STORAGE_KEY);
  } catch {
    return null;
  }
}

function writeLast(name: ArtworkName): void {
  try {
    localStorage.setItem(STORAGE_KEY, name);
  } catch {
    // Private windows can refuse storage; the next pick may repeat.
  }
}

export function useAuthArtworkController(random: () => number = Math.random) {
  const artwork = pickArtwork(readLast(), random);
  writeLast(artwork.name);
  return { artwork };
}
