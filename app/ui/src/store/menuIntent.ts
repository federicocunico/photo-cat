// Azioni chieste dalla barra dei menu a una pagina che le esegue con i propri strumenti
// (application-menu, D6): ad esempio File › "Importa una cartella…" apre Importa e poi il suo
// selettore di cartelle, come il pulsante della pagina.
import { create } from "zustand";

export type MenuIntent = "import.pick" | "rawReview.project";

export const useMenuIntent = create<{
  intent: MenuIntent | null;
  request: (intent: MenuIntent) => void;
  /** La pagina prende l'intento che la riguarda (una volta sola). */
  take: (intent: MenuIntent) => boolean;
}>((set, get) => ({
  intent: null,
  request: (intent) => set({ intent }),
  take: (intent) => {
    if (get().intent !== intent) return false;
    set({ intent: null });
    return true;
  },
}));

/** Finestra Informazioni su photo-cat: si apre dal menu Aiuto, dalla palette e su macOS dal menu dell'app. */
export const useAboutDialog = create<{ open: boolean; setOpen: (open: boolean) => void }>(
  (set) => ({
    open: false,
    setOpen: (open) => set({ open }),
  }),
);
