// Trascinamento di foto con gli eventi del puntatore (library-catalog-organize, decisione 2).
// Non si usa il drag and drop HTML5: su Windows Tauri lo intercetta. Un trascinamento parte
// da `armPhotoDrag` (pointerdown su una miniatura) dopo uno spostamento di DRAG_THRESHOLD px;
// i bersagli sono elementi con `data-drop-id` registrati con `useDropTarget`.
import { useEffect, useRef } from "react";
import { create } from "zustand";
import { useTranslation } from "react-i18next";
import type { Target } from "../../lib/ipc";

export const DRAG_THRESHOLD = 5;

export type DragPayload = { target: Target; count: number };

type DragState = {
  payload: DragPayload | null;
  x: number;
  y: number;
  /** Id del bersaglio sotto il puntatore. */
  over: string | null;
};

export const usePhotoDrag = create<DragState>(() => ({ payload: null, x: 0, y: 0, over: null }));

type Handler = { accepts: boolean; onDrop: (payload: DragPayload) => void };
const handlers = new Map<string, Handler>();

let suppressClick = false;
let session: { cancel: () => void } | null = null;

/** Vero (una sola volta) per il `click` che segue subito un trascinamento: va ignorato. */
export function consumeDragClick(): boolean {
  const was = suppressClick;
  suppressClick = false;
  return was;
}

const dropIdAt = (x: number, y: number): string | null =>
  document.elementFromPoint?.(x, y)?.closest<HTMLElement>("[data-drop-id]")?.dataset.dropId ?? null;

/** Annulla il trascinamento in corso (nessuna modifica). */
export function cancelPhotoDrag() {
  session?.cancel();
}

/**
 * Prepara un trascinamento dal punto di pressione: parte solo dopo DRAG_THRESHOLD px, così
 * un clic con pochi pixel di tremolio resta un clic.
 */
export function armPhotoDrag(from: { x: number; y: number }, payload: DragPayload) {
  session?.cancel();
  let active = false;
  let cancelled = false;
  let frame = 0;
  const body = document.body;

  const hit = (x: number, y: number) => {
    const over = dropIdAt(x, y);
    if (usePhotoDrag.getState().over !== over) usePhotoDrag.setState({ over });
  };
  const onMove = (e: PointerEvent) => {
    const { clientX: x, clientY: y } = e;
    if (!active) {
      if (Math.hypot(x - from.x, y - from.y) < DRAG_THRESHOLD) return;
      active = true;
      body.dataset.photoDrag = "";
      usePhotoDrag.setState({ payload, x, y, over: null });
    }
    if (cancelled) return;
    usePhotoDrag.setState({ x, y });
    if (!frame)
      frame = requestAnimationFrame(() => {
        frame = 0;
        const s = usePhotoDrag.getState();
        if (s.payload) hit(s.x, s.y);
      });
  };
  const finish = () => {
    window.removeEventListener("pointermove", onMove);
    window.removeEventListener("pointerup", onUp);
    window.removeEventListener("pointercancel", onCancelEvent);
    window.removeEventListener("keydown", onKey, true);
    if (frame) cancelAnimationFrame(frame);
    delete body.dataset.photoDrag;
    usePhotoDrag.setState({ payload: null, over: null });
    if (session === handle) session = null;
    if (active) {
      suppressClick = true;
      // Se il rilascio non produce un click, il flag non deve restare per il clic dopo.
      setTimeout(() => (suppressClick = false), 0);
    }
  };
  const onUp = (e: PointerEvent) => {
    const drop = active && !cancelled ? dropIdAt(e.clientX, e.clientY) : null;
    const target = drop ? handlers.get(drop) : undefined;
    finish();
    if (target?.accepts) target.onDrop(payload);
  };
  const onCancelEvent = () => {
    cancelled = true;
    finish();
  };
  const onKey = (e: KeyboardEvent) => {
    if (e.key !== "Escape" || !active) return;
    e.preventDefault();
    e.stopPropagation();
    // Esc ferma il trascinamento; il rilascio successivo chiude la sessione senza agire.
    cancelled = true;
    usePhotoDrag.setState({ payload: null, over: null });
  };
  const handle = {
    cancel: () => {
      cancelled = true;
      finish();
    },
  };
  session = handle;
  window.addEventListener("pointermove", onMove);
  window.addEventListener("pointerup", onUp);
  window.addEventListener("pointercancel", onCancelEvent);
  window.addEventListener("keydown", onKey, true);
}

/** Etichetta "N foto" vicino al puntatore durante un trascinamento. */
export function DragGhost() {
  const { t } = useTranslation();
  const payload = usePhotoDrag((s) => s.payload);
  const x = usePhotoDrag((s) => s.x);
  const y = usePhotoDrag((s) => s.y);
  if (!payload) return null;
  return (
    <div aria-hidden="true" className="drag-ghost" style={{ left: x + 14, top: y + 14 }}>
      {t("dnd.ghost", { count: payload.count })}
    </div>
  );
}

/**
 * Rende una riga bersaglio di rilascio. Restituisce gli attributi da applicare alla riga:
 * evidenziata (`data-drop-over`) se sotto il puntatore e accetta, `data-drop-refused`
 * (cursore not-allowed) durante un trascinamento se non accetta.
 */
export function useDropTarget({
  id,
  accepts,
  onDrop,
}: {
  id: string;
  accepts: boolean;
  onDrop: (payload: DragPayload) => void;
}) {
  const latest = useRef<Handler>({ accepts, onDrop });
  latest.current = { accepts, onDrop };
  useEffect(() => {
    handlers.set(id, {
      get accepts() {
        return latest.current.accepts;
      },
      onDrop: (p) => latest.current.onDrop(p),
    });
    return () => {
      handlers.delete(id);
    };
  }, [id]);
  const dragging = usePhotoDrag((s) => s.payload != null);
  const over = usePhotoDrag((s) => s.over === id);
  return {
    "data-drop-id": id,
    "data-drop-over": (dragging && over && accepts) || undefined,
    "data-drop-refused": (dragging && !accepts) || undefined,
  };
}
