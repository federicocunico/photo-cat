// Scorciatoie del modulo Varianti RAW (design D5). Si registrano solo con una sessione aperta,
// cioè quando il modulo è a schermo; il registro le ignora nei campi di testo e con un dialogo
// aperto. Ctrl+1–9 restano alla navigazione dei moduli.
import { useRef } from "react";
import { useTranslation } from "react-i18next";
import { useCommands, type Command } from "../../commands/registry";
import type { ReviewShotView } from "../../lib/ipc";
import { chooseWinner, selectShot, setRating, useRawReview } from "../../store/rawReview";
import { canChoose } from "./choose";
import { moveFocus, type Arrow } from "./panelFocus";

type Context = {
  shot: ReviewShotView | undefined;
  panels: readonly string[];
  focused: number;
  onFocus: (index: number) => void;
};

/**
 * Il fuoco è su un controllo che usa già Spazio, Invio o le frecce (pulsante, collegamento,
 * riepilogo, elenco degli scatti): lì la tastiera è del controllo. Un pulsante toccato col
 * mouse non conta (non è `:focus-visible`), così Spazio continua a scorrere gli scatti.
 */
export function focusOwnsKeys(active: Element | null = document.activeElement): boolean {
  if (!(active instanceof HTMLElement) || active === document.body) return false;
  if (active.closest("[data-rr-shots]")) return true;
  if (!active.matches("button, a[href], summary, input, [role='checkbox']")) return false;
  try {
    return active.matches(":focus-visible");
  } catch {
    return true;
  }
}

export function useRawReviewShortcuts(context: Context, enabled: boolean) {
  const { t } = useTranslation();
  const ref = useRef({ context, t });
  ref.current = { context, t };

  const ready = () => ref.current.context.shot != null;
  const chooseAt = (position: number) => {
    const { shot, panels } = ref.current.context;
    const preset = panels[position];
    if (!shot || !preset) return;
    if (canChoose(shot, preset, ref.current.t)) void chooseWinner(shot.key, preset);
  };
  const rate = (n: number) => {
    const { shot } = ref.current.context;
    if (shot) void setRating(shot.key, n);
  };
  const go = (delta: number) => {
    selectShot(useRawReview.getState().shotIndex + delta);
  };
  const base = { group: "rawReview" as const, palette: false };

  const commands: Command[] = [
    ...[1, 2, 3, 4, 5, 6, 7, 8, 9].map(
      (n): Command => ({
        ...base,
        id: `rawReview.choose.${n}`,
        label: t("rawReviewShortcuts.choose", { n }),
        shortcut: { key: String(n), shift: true },
        when: ready,
        run: () => chooseAt(n - 1),
      }),
    ),
    {
      ...base,
      id: "rawReview.chooseFocused",
      label: t("rawReviewShortcuts.chooseFocused"),
      shortcut: { key: "Enter" },
      when: () => ready() && !focusOwnsKeys(),
      run: () => chooseAt(ref.current.context.focused),
    },
    ...[0, 1, 2, 3, 4, 5].flatMap((n): Command[] => [
      {
        ...base,
        id: `rawReview.rate.${n}`,
        label: t("rawReviewShortcuts.rate", { count: n }),
        shortcut: { key: String(n) },
        when: ready,
        run: () => rate(n),
      },
      {
        ...base,
        id: `rawReview.rateAlt.${n}`,
        label: t("rawReviewShortcuts.rate", { count: n }),
        shortcut: { key: String(n), alt: true },
        when: ready,
        run: () => rate(n),
      },
    ]),
    {
      ...base,
      id: "rawReview.nextShot",
      label: t("rawReviewCompare.nextShot"),
      shortcut: { key: " " },
      when: () => !focusOwnsKeys(),
      run: () => go(1),
    },
    {
      ...base,
      id: "rawReview.previousShot",
      label: t("rawReviewCompare.previousShot"),
      shortcut: { key: " ", shift: true },
      when: () => !focusOwnsKeys(),
      run: () => go(-1),
    },
    ...(["ArrowLeft", "ArrowRight", "ArrowUp", "ArrowDown"] as Arrow[]).map(
      (key): Command => ({
        ...base,
        id: `rawReview.focus.${key}`,
        label: t("rawReviewShortcuts.focusPanel"),
        shortcut: { key },
        when: () => ready() && !focusOwnsKeys(),
        run: () => {
          const { panels, focused, onFocus } = ref.current.context;
          onFocus(moveFocus(focused, panels.length, key));
        },
      }),
    ),
  ];
  useCommands(commands, enabled);
}
