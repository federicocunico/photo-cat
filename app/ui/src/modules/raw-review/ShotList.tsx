// Elenco degli scatti della sessione (barra laterale): miniatura, nome, stelle e variante
// scelta. È virtualizzato (160 scatti o più) e chiede le miniature solo per le righe visibili.
// La selezione multipla usa `useGridSelection`: gli id sono le posizioni degli scatti
// nell'elenco della sessione, che non cambia finché la sessione resta aperta.
import { convertFileSrc } from "@tauri-apps/api/core";
import { useVirtualizer } from "@tanstack/react-virtual";
import {
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
  type KeyboardEvent,
  type MouseEvent,
} from "react";
import { useTranslation } from "react-i18next";
import { useActiveGrid } from "../../commands/gridShortcuts";
import { Button } from "../../components/ui";
import { commands } from "../../lib/ipc";
import type { ReviewShotView, ShotDecision } from "../../lib/ipc";
import { useGridSelection } from "../../lib/useGridSelection";
import { selHas } from "../../store/library";
import { notifyError } from "../../store/toasts";
import { presetLabel } from "./presetLabel";

/** Altezza di una riga (miniatura 3:2 di 40 px più i margini). */
export const SHOT_ROW_HEIGHT = 56;
/** Righe fuori vista che si preparano sopra e sotto. */
const OVERSCAN = 6;
/** Salto dei tasti Pag↑ e Pag↓. */
const PAGE_STEP = 8;
/** Attesa prima di chiedere le miniature: scorrendo in fretta si chiedono solo le ultime. */
const THUMBNAIL_DEBOUNCE_MS = 80;
/** Dimensioni dell'area finché non è misurata (e nei test). */
const FALLBACK = { width: 220, height: 600 };

type Props = {
  sessionId: string;
  shots: readonly ReviewShotView[];
  decisions: Record<string, ShotDecision> | undefined;
  /** Posizione dello scatto mostrato. */
  current: number;
  onOpen: (index: number) => void;
  /** Scatti selezionati (per "Sviluppa selezionati"): chiavi nell'ordine dell'elenco. */
  onSelectionChange?: (shotKeys: string[]) => void;
};

type Scope = { sessionId: string };

export function ShotList({
  sessionId,
  shots,
  decisions,
  current,
  onOpen,
  onSelectionChange,
}: Props) {
  const { t } = useTranslation();
  const scrollRef = useRef<HTMLDivElement>(null);
  const [focused, setFocused] = useState(false);
  const total = shots.length;

  const order = useMemo(() => shots.map((_, i) => i), [shots]);
  const selection = useGridSelection<Scope>({
    filter: { sessionId },
    order,
    // Gli scatti sono già in memoria: la base dell'insieme completo si calcola qui.
    fetchBase: async () => ({ count: total, maxId: total - 1 }),
    onError: notifyError,
  });

  // Seleziona tutto, Inverti ed Esc passano dalla shell alla lista che ha il fuoco.
  const gridRef = useRef(selection);
  gridRef.current = selection;
  useActiveGrid(
    {
      selectAll: () => gridRef.current.selectAll(),
      invert: () => gridRef.current.invert(),
      clear: () => gridRef.current.clear(),
      count: () => gridRef.current.count,
    },
    focused,
  );

  const selectedKeys = useMemo(() => {
    const keys: string[] = [];
    shots.forEach((shot, i) => {
      if (selHas(selection.selection, i)) keys.push(shot.key);
    });
    return keys;
  }, [shots, selection.selection]);
  const notify = useRef(onSelectionChange);
  notify.current = onSelectionChange;
  useEffect(() => {
    notify.current?.(selectedKeys);
  }, [selectedKeys]);

  const virt = useVirtualizer({
    count: total,
    getScrollElement: () => scrollRef.current,
    estimateSize: () => SHOT_ROW_HEIGHT,
    overscan: OVERSCAN,
    initialRect: FALLBACK,
    // In jsdom l'altezza misurata è 0: resta quella di partenza.
    observeElementRect: (_instance, cb) => {
      const el = scrollRef.current;
      const emit = () => {
        cb({
          width: el?.clientWidth || FALLBACK.width,
          height: el?.clientHeight || FALLBACK.height,
        });
      };
      emit();
      if (!el || typeof ResizeObserver === "undefined") return () => {};
      const observer = new ResizeObserver(emit);
      observer.observe(el);
      return () => observer.disconnect();
    },
  });
  const items = virt.getVirtualItems();

  // Lo scatto mostrato resta in vista (frecce, Spazio, pulsanti del piede).
  useEffect(() => {
    if (total > 0) virt.scrollToIndex(current, { align: "auto" });
    // `virt` cambia a ogni render: conta solo lo scatto.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [current, total]);

  const thumbs = useThumbnails(
    sessionId,
    shots,
    items.map((i) => i.index),
  );

  const openRow = (index: number, event: MouseEvent) => {
    const ctrl = event.ctrlKey || event.metaKey;
    selection.click(index, { ctrl, shift: event.shiftKey });
    // Ctrl e Maiusc compongono la selezione; il clic semplice apre anche lo scatto.
    if (!ctrl && !event.shiftKey) onOpen(index);
  };

  const onKeyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    if (event.ctrlKey || event.metaKey || event.altKey || total === 0) return;
    const last = total - 1;
    const target =
      event.key === "ArrowDown"
        ? Math.min(last, current + 1)
        : event.key === "ArrowUp"
          ? Math.max(0, current - 1)
          : event.key === "Home"
            ? 0
            : event.key === "End"
              ? last
              : event.key === "PageDown"
                ? Math.min(last, current + PAGE_STEP)
                : event.key === "PageUp"
                  ? Math.max(0, current - PAGE_STEP)
                  : null;
    if (target === null) return;
    event.preventDefault();
    if (event.shiftKey) selection.click(target, { ctrl: false, shift: true });
    else selection.setIds([target], target);
    onOpen(target);
  };

  const label = useCallback(
    (shot: ReviewShotView, decision: ShotDecision | undefined) => {
      const parts = [shot.displayName];
      if (decision?.rating) parts.push(t("rawReviewShots.stars", { count: decision.rating }));
      if (decision?.winnerPreset)
        parts.push(t("rawReviewShots.best", { name: presetLabel(t, decision.winnerPreset) }));
      if (decision?.reviewed) parts.push(t("rawReviewShots.reviewed"));
      return parts.join(", ");
    },
    [t],
  );

  return (
    <aside className="rr-shots" aria-label={t("rawReviewShots.region")}>
      <div className="rr-shots-head">
        <strong>{t("rawReviewShots.title")}</strong>
        <span className="muted" aria-live="polite">
          {selection.count > 0 ? t("rawReviewShots.selected", { count: selection.count }) : ""}
        </span>
        {selection.count > 0 && (
          <Button size="sm" variant="ghost" onClick={() => selection.clear()}>
            {t("rawReviewShots.clearSelection")}
          </Button>
        )}
      </div>
      <div
        ref={scrollRef}
        className="rr-shots-scroll"
        data-rr-shots=""
        data-testid="rr-shots"
        role="listbox"
        aria-multiselectable="true"
        aria-label={t("rawReviewShots.list")}
        aria-activedescendant={total > 0 ? `rr-shot-${current}` : undefined}
        tabIndex={0}
        onFocus={() => setFocused(true)}
        onBlur={() => setFocused(false)}
        onKeyDown={onKeyDown}
      >
        <div className="rr-shots-inner" style={{ height: virt.getTotalSize() }}>
          {items.map((item) => {
            const shot = shots[item.index];
            const decision = decisions?.[shot.key];
            const thumb = thumbs[shot.key];
            const rating = decision?.rating ?? 0;
            const winner = decision?.winnerPreset;
            const picked = selection.has(item.index);
            return (
              <div
                key={shot.key}
                id={`rr-shot-${item.index}`}
                role="option"
                aria-selected={picked}
                aria-current={item.index === current ? "true" : undefined}
                aria-label={label(shot, decision)}
                data-index={item.index}
                className={`rr-shot${item.index === current ? " is-current" : ""}${
                  picked ? " is-selected" : ""
                }`}
                style={{ transform: `translateY(${item.start}px)`, height: item.size }}
                onClick={(event) => openRow(item.index, event)}
              >
                <span className="rr-shot-thumb" aria-hidden="true">
                  {thumb && <img src={thumb} alt="" decoding="async" draggable={false} />}
                </span>
                <span className="rr-shot-text" aria-hidden="true">
                  <span className="rr-shot-name">{shot.displayName}</span>
                  <span className="rr-shot-meta">
                    <span className="rr-shot-stars">
                      {"★".repeat(rating)}
                      {"☆".repeat(5 - rating)}
                    </span>
                    {winner && <span className="rr-shot-best">{presetLabel(t, winner)}</span>}
                    {decision?.reviewed && !winner && <span className="rr-shot-seen">✓</span>}
                  </span>
                </span>
              </div>
            );
          })}
        </div>
      </div>
    </aside>
  );
}

/**
 * Miniature degli scatti in vista. Ogni scatto si chiede una volta sola, e ogni richiesta
 * riguarda le righe visibili del momento (il backend sostituisce la coda precedente).
 */
function useThumbnails(
  sessionId: string,
  shots: readonly ReviewShotView[],
  visible: number[],
): Record<string, string> {
  const [loaded, setLoaded] = useState<{ id: string; urls: Record<string, string> }>({
    id: sessionId,
    urls: {},
  });
  const asked = useRef<{ id: string; keys: Set<string> }>({ id: sessionId, keys: new Set() });
  if (asked.current.id !== sessionId) asked.current = { id: sessionId, keys: new Set() };

  const signature = visible
    .map((i) => shots[i]?.key)
    .filter(Boolean)
    .join("\n");
  useEffect(() => {
    const keys = signature ? signature.split("\n") : [];
    const seen = asked.current;
    if (keys.every((k) => seen.keys.has(k))) return;
    const timer = window.setTimeout(() => {
      const fresh = keys.filter((k) => !seen.keys.has(k));
      for (const k of fresh) seen.keys.add(k);
      commands
        .rawReviewThumbnails(sessionId, keys)
        .then((rows) => {
          setLoaded((prev) => {
            const urls = prev.id === sessionId ? { ...prev.urls } : {};
            for (const row of rows ?? []) {
              if (row.key) urls[row.shotKey] = convertFileSrc(row.key, "review");
            }
            return { id: sessionId, urls };
          });
        })
        .catch(() => {
          // Senza miniatura la riga mostra il segnaposto; si riprova al prossimo scorrimento.
          for (const k of fresh) seen.keys.delete(k);
        });
    }, THUMBNAIL_DEBOUNCE_MS);
    return () => window.clearTimeout(timer);
  }, [sessionId, signature]);

  return loaded.id === sessionId ? loaded.urls : NO_THUMBS;
}
const NO_THUMBS: Record<string, string> = {};
