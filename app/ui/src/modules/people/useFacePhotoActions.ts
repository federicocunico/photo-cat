// Dal volto alla foto: "Apri con", "Mostra nella cartella" e "Mostra in Libreria"
// sui media distinti di una selezione di volti (design D5).
import { useQueryClient } from "@tanstack/react-query";
import { useState } from "react";
import {
  commands,
  type FaceSelectionMedia,
  type FaceView,
  type MediaDetail,
  type OpenTarget,
} from "../../lib/ipc";
import { useLayout } from "../../store/layout";
import { emptyFilter, useLibrary } from "../../store/library";
import { useOpenWith } from "../library/useOpenWith";

/** Un volto per foto, nell'ordine di comparsa: 4 volti di 3 foto restano 3. */
export function distinctByMedia(faces: FaceView[]): FaceView[] {
  const seen = new Set<number>();
  return faces.filter((f) => !seen.has(f.mediaId) && seen.add(f.mediaId));
}

/** Cartella della foto, con separatori di Windows (`D:\Foto\x.jpg`) o Unix. */
export const folderOf = (path: string) => path.replace(/[\\/][^\\/]*$/, "") || path;

/** Cartella e foto selezionata nella Libreria, che si apre sulla foto del volto. */
export function showInLibrary(detail: MediaDetail) {
  const lib = useLibrary.getState();
  lib.setFilter({
    ...emptyFilter(),
    folder: { path: folderOf(detail.item.path), recursive: false },
  });
  lib.setSelection([detail.item.id], detail.item.id);
  useLayout.getState().openModule({ module: "library" });
}

export function useFacePhotoActions() {
  const qc = useQueryClient();
  const openWith = useOpenWith();
  /** Percorsi dei file non più sul disco, mostrati come avviso. */
  const [missing, setMissing] = useState<string[]>([]);

  const detailOf = (media: number) =>
    qc.fetchQuery({
      queryKey: ["media-detail", media],
      queryFn: () => commands.mediaDetail(media),
      staleTime: 30_000,
    });

  /** Apre i media distinti dei volti; quelli mancanti producono l'avviso e non partono. */
  const open = async (faces: FaceView[], target: OpenTarget) => {
    setMissing([]);
    const details = await Promise.all(distinctByMedia(faces).map((f) => detailOf(f.mediaId)));
    const gone = details.filter((d) => d.item.status === "missing");
    setMissing(gone.map((d) => d.item.path));
    const ids = details.filter((d) => d.item.status !== "missing").map((d) => d.item.id);
    if (ids.length) await openWith.open(ids, target);
  };

  /** Come `open`, su foto già risolte dal backend (selezione per filtro oltre i volti caricati). */
  const openResolved = async (media: FaceSelectionMedia, target: OpenTarget) => {
    setMissing(media.missing);
    if (media.present.length) await openWith.open(media.present, target);
  };

  const library = async (face: FaceView) => showInLibrary(await detailOf(face.mediaId));

  return {
    open,
    openResolved,
    library,
    detailOf,
    missing,
    clearMissing: () => setMissing([]),
    error: openWith.error,
    clearError: openWith.clearError,
  };
}
