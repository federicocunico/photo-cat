// Metadati di tutte le varianti dello scatto mostrato (intestazioni dei pannelli e barra delle
// informazioni). I file di una sessione non cambiano mentre è aperta: ogni lettura si fa una volta.
import { useEffect, useState } from "react";
import { commands } from "../../lib/ipc";
import type { ImageMetadata, ReviewShotView } from "../../lib/ipc";

const cache = new Map<string, Promise<ImageMetadata>>();

function load(sessionId: string, shotKey: string, preset: string): Promise<ImageMetadata> {
  const id = `${sessionId}|${shotKey}|${preset}`;
  let entry = cache.get(id);
  if (!entry) {
    entry = commands.rawReviewMetadata(sessionId, shotKey, preset);
    entry.catch(() => cache.delete(id));
    cache.set(id, entry);
  }
  return entry;
}

/** Dimentica le letture di una sessione chiusa. */
export function forgetMetadata(sessionId: string): void {
  for (const key of cache.keys()) if (key.startsWith(`${sessionId}|`)) cache.delete(key);
}

/** Metadati per preset dello scatto; una variante senza metadati leggibili resta assente. */
export function useShotMetadata(
  sessionId: string | undefined,
  shot: ReviewShotView | undefined,
): Record<string, ImageMetadata> {
  const [byPreset, setByPreset] = useState<Record<string, ImageMetadata>>({});
  useEffect(() => {
    setByPreset({});
    if (!sessionId || !shot) return;
    let current = true;
    for (const variant of shot.variants) {
      load(sessionId, shot.key, variant.preset)
        .then((metadata) => {
          if (current) setByPreset((prev) => ({ ...prev, [variant.preset]: metadata }));
        })
        .catch(() => undefined);
    }
    return () => {
      current = false;
    };
  }, [sessionId, shot]);
  return byPreset;
}
