// Barra delle azioni sulla selezione (ui-foundations, "Barre dei comandi"; ui-usability-audit D2):
// una sola riga, il testo non esce mai dai pulsanti; le azioni che non entrano passano, dalla
// meno importante, nel menu "Altro". Le larghezze si misurano su una copia nascosta della riga,
// e si rimisurano solo quando cambiano lo spazio o le azioni.
import { useLayoutEffect, useRef, useState, type ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "./Button";
import { Menu, type MenuItem } from "./Menu";
import { Icon } from "../icons";

export type BarAction = {
  id: string;
  label: string;
  icon?: ReactNode;
  onSelect?: () => unknown;
  /** 1 è la più importante: le azioni con il numero più alto passano per prime in "Altro". */
  priority?: number;
  variant?: "primary" | "danger" | "ghost";
  disabled?: boolean;
  /** Solo nel menu "Altro", mai come pulsante (azioni rare o distruttive). */
  menuOnly?: boolean;
  /** Pulsante con un menu a tendina; in "Altro" diventa un sottomenu. */
  items?: MenuItem[];
  /**
   * Elemento proprio sempre visibile (es. un selettore con ricerca), che non passa mai in
   * "Altro": la sua larghezza riduce lo spazio delle altre azioni.
   */
  render?: () => ReactNode;
};

const GAP = 8;

/** Quante azioni (in ordine di priorità) entrano in `available` px, con "Altro" se serve. */
export function fitActions(widths: number[], available: number, more: number): number {
  const total = widths.reduce((a, w) => a + w + GAP, 0);
  if (total <= available) return widths.length;
  let used = more + GAP;
  let n = 0;
  for (const w of widths) {
    if (used + w + GAP > available) break;
    used += w + GAP;
    n++;
  }
  return n;
}

export function ActionBar({
  label,
  lead,
  actions,
  end,
  menu = [],
  className,
}: {
  /** Nome della barra per il lettore di schermo. */
  label: string;
  /** Testo o comandi all'inizio della barra (es. "3 selezionati"). */
  lead?: ReactNode;
  actions: BarAction[];
  /** Comando alla fine della barra (es. Deseleziona). */
  end?: ReactNode;
  /** Voci fisse del menu "Altro", dopo le azioni che non entrano. */
  menu?: MenuItem[];
  className?: string;
}) {
  const { t } = useTranslation();
  const bar = useRef<HTMLDivElement>(null);
  const measure = useRef<HTMLDivElement>(null);
  const movable = actions.filter((a) => !a.menuOnly && !a.render);
  const byPriority = [...movable].sort((a, b) => (a.priority ?? 1) - (b.priority ?? 1));
  const [fit, setFit] = useState(movable.length);
  const signature = actions.map((a) => `${a.id}:${a.label}:${a.disabled ? 1 : 0}`).join("|");

  useLayoutEffect(() => {
    const el = bar.current;
    const ruler = measure.current;
    if (!el || !ruler) return;
    let frame = 0;
    const compute = () => {
      const width = el.getBoundingClientRect().width;
      // Senza misure (finestra nascosta) si mostra tutto: meglio che nascondere azioni a caso.
      if (width <= 0) return setFit(byPriority.length);
      const widthOf = (root: HTMLElement, id: string) =>
        root.querySelector<HTMLElement>(`[data-measure="${id}"]`)?.getBoundingClientRect().width ??
        0;
      const pinned = actions.filter((a) => a.render).map((a) => widthOf(el, a.id) + GAP);
      const fixed =
        ["lead", "end"].reduce((a, id) => a + widthOf(el, id) + GAP, 0) +
        pinned.reduce((a, w) => a + w, 0);
      setFit(
        fitActions(
          byPriority.map((a) => widthOf(ruler, a.id)),
          width - fixed,
          widthOf(ruler, "more"),
        ),
      );
    };
    compute();
    const ro = new ResizeObserver(() => {
      cancelAnimationFrame(frame);
      frame = requestAnimationFrame(compute);
    });
    ro.observe(el);
    return () => {
      cancelAnimationFrame(frame);
      ro.disconnect();
    };
    // La misura dipende solo dalle azioni mostrate e dai loro testi.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [signature]);

  const shownIds = new Set(byPriority.slice(0, fit).map((a) => a.id));
  const overflow = actions.filter((a) => !a.render && (a.menuOnly || !shownIds.has(a.id)));
  const overflowItems: MenuItem[] = overflow.map((a) =>
    a.items
      ? { kind: "submenu", label: a.label, icon: a.icon, items: a.items }
      : {
          label: a.label,
          icon: a.icon,
          danger: a.variant === "danger",
          disabled: a.disabled,
          onSelect: () => a.onSelect?.(),
        },
  );
  const moreItems: MenuItem[] =
    overflowItems.length && menu.length
      ? [...overflowItems, { kind: "separator" }, ...menu]
      : [...overflowItems, ...menu];
  const button = (a: BarAction, measuring = false) => {
    const content = (
      <Button
        key={a.id}
        size="sm"
        variant={a.variant}
        icon={a.icon}
        disabled={a.disabled}
        tabIndex={measuring ? -1 : undefined}
        data-measure={measuring ? a.id : undefined}
        onClick={measuring || a.items ? undefined : () => void a.onSelect?.()}
      >
        {a.label}
        {a.items && <Icon.ChevronDown size={14} aria-hidden="true" />}
      </Button>
    );
    return a.items && !measuring ? <Menu key={a.id} trigger={content} items={a.items} /> : content;
  };
  const more = (measuring = false) => (
    <Button
      size="sm"
      variant="ghost"
      icon={<Icon.More size={16} />}
      tabIndex={measuring ? -1 : undefined}
      data-measure={measuring ? "more" : undefined}
    >
      {t("common.more")}
    </Button>
  );

  return (
    <div
      ref={bar}
      role="toolbar"
      aria-label={label}
      className={["ui-action-bar", className].filter(Boolean).join(" ")}
    >
      {lead && (
        <span className="ui-action-bar-lead" data-measure="lead">
          {lead}
        </span>
      )}
      {actions.map((a) =>
        a.render ? (
          <span key={a.id} className="ui-action-bar-pinned" data-measure={a.id}>
            {a.render()}
          </span>
        ) : shownIds.has(a.id) && !a.menuOnly ? (
          button(a)
        ) : null,
      )}
      {moreItems.length > 0 && <Menu trigger={more()} items={moreItems} align="end" />}
      {end && (
        <span className="ui-action-bar-end" data-measure="end">
          {end}
        </span>
      )}
      {/* Copia nascosta per misurare le larghezze naturali (non raggiungibile). */}
      <div ref={measure} className="ui-action-bar-ruler" aria-hidden="true">
        {movable.map((a) => button(a, true))}
        {more(true)}
      </div>
    </div>
  );
}
