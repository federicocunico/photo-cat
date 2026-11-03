import { create } from "zustand";

export type NoticeTone = "info" | "ok" | "error";

export type Notice = {
  id: number;
  message: string;
  tone: NoticeTone;
  /** Millisecondi prima della chiusura automatica; `null`: resta finché non si chiude. */
  duration: number | null;
  action?: { label: string; run: () => unknown };
  /**
   * Avvisi con la stessa chiave si sostituiscono (es. l'esito dell'ultima esclusione),
   * invece di accumularsi.
   */
  key?: string;
  /** Chiamata quando l'avviso sparisce (chiusura, timeout, sostituzione o azione). */
  onDismiss?: () => void;
};

/** Al massimo tanti avvisi visibili: i più vecchi lasciano il posto. */
export const MAX_NOTICES = 4;

let next = 0;

type ToastState = {
  notices: Notice[];
  dismiss: (id: number) => void;
};

function without(notices: Notice[], drop: (n: Notice) => boolean) {
  const kept: Notice[] = [];
  for (const n of notices) {
    if (drop(n)) n.onDismiss?.();
    else kept.push(n);
  }
  return kept;
}

export const useToast = create<ToastState>((set, get) => ({
  notices: [],
  dismiss: (id) => set({ notices: without(get().notices, (n) => n.id === id) }),
}));

export type NotifyInput = Omit<Notice, "id" | "duration" | "tone"> & {
  tone?: NoticeTone;
  duration?: number | null;
};

/**
 * Mostra un avviso nell'unico `ToastHost`. Gli errori restano finché non si chiudono
 * (WCAG 2.2.1); gli altri si chiudono da soli dopo 8 secondi, con pausa al passaggio.
 */
export function notify(input: NotifyInput): number {
  const tone = input.tone ?? "info";
  const notice: Notice = {
    ...input,
    tone,
    id: ++next,
    duration: input.duration !== undefined ? input.duration : tone === "error" ? null : 8000,
  };
  const current = useToast.getState().notices;
  let notices = input.key ? without(current, (n) => n.key === input.key) : current;
  notices = [...notices, notice];
  if (notices.length > MAX_NOTICES) {
    const overflow = notices.length - MAX_NOTICES;
    notices.slice(0, overflow).forEach((n) => n.onDismiss?.());
    notices = notices.slice(overflow);
  }
  useToast.setState({ notices });
  return notice.id;
}

/** Errore come avviso: testo leggibile dall'errore del motore (`detail`) o generico. */
export function notifyError(error: unknown, fallback?: string): number {
  const detail = (error as { detail?: string } | null)?.detail;
  return notify({ message: detail ?? fallback ?? String(error), tone: "error" });
}

/** Chiude l'avviso con quella chiave, se c'è. */
export function dismissKey(key: string) {
  const s = useToast.getState();
  useToast.setState({ notices: without(s.notices, (n) => n.key === key) });
}
