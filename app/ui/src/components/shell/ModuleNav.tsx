import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { useQuery } from "@tanstack/react-query";
import { MODULES, NAV_GROUPS, moduleForShortcut, type NavItem } from "../../modules/registry";
import { useLayout, type ModuleId } from "../../store/layout";
import { activeJobs, useLive } from "../../store/live";
import { commands } from "../../lib/ipc";
import { isTyping, modalOpen } from "../../lib/keys";
import { fmtInt } from "../../lib/format";
import { Icon } from "../icons";
import { Tooltip } from "../ui";
import "./shell.css";

/** Sotto questa larghezza della finestra la navigazione passa a sole icone (design D2). */
export const NAV_COMPACT_BELOW = 1008;

const MOD = /Mac|iPhone|iPad/.test(globalThis.navigator?.platform ?? "") ? "⌘" : "Ctrl";

/** Finestra stretta: icone senza toccare la preferenza salvata per le finestre larghe. */
function useNarrowWindow() {
  const query = `(max-width: ${NAV_COMPACT_BELOW - 1}px)`;
  const [narrow, setNarrow] = useState(
    () => typeof window !== "undefined" && !!window.matchMedia?.(query).matches,
  );
  useEffect(() => {
    const mq = window.matchMedia?.(query);
    if (!mq) return;
    const update = () => setNarrow(mq.matches);
    update();
    mq.addEventListener?.("change", update);
    return () => mq.removeEventListener?.("change", update);
  }, [query]);
  return narrow;
}

/** Ctrl+1…7 apre le voci dei gruppi, Ctrl+B riduce o espande la navigazione. */
function useNavHotkeys(open: (m: ModuleId) => void) {
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.defaultPrevented || e.altKey || e.shiftKey || !(e.ctrlKey || e.metaKey)) return;
      if (isTyping(e.target) || modalOpen()) return;
      if (e.key === "b" || e.key === "B") {
        e.preventDefault();
        useLayout.getState().toggleNav();
        return;
      }
      const target = /^[1-9]$/.test(e.key) ? moduleForShortcut(Number(e.key)) : undefined;
      if (target) {
        e.preventDefault();
        open(target);
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [open]);
}

/** Conteggi accanto alle voci: persone da rivedere e attività in corso. */
function useBadges(): Partial<Record<ModuleId, number>> {
  const running = useLive((s) => activeJobs(s.jobs).length);
  const sections = useQuery({
    queryKey: ["people-sections"],
    queryFn: () => commands.peopleSections(),
    staleTime: 30_000,
  });
  return { people: sections.data?.review ?? 0, jobs: running };
}

export function ModuleNav() {
  const { t } = useTranslation();
  const module = useLayout((s) => s.module);
  const setModule = useLayout((s) => s.setModule);
  const preferCompact = useLayout((s) => s.navCompact);
  const toggleNav = useLayout((s) => s.toggleNav);
  const narrow = useNarrowWindow();
  const compact = preferCompact || narrow;
  const badges = useBadges();
  useNavHotkeys(setModule);

  const badgeText = (id: ModuleId, n: number) =>
    t(id === "people" ? "nav.badgeReview" : "nav.badgeRunning", { count: n });

  const item = ({ id, icon: I, shortcut }: NavItem) => {
    const active = id === module;
    const name = t(`modules.${id}`);
    const count = badges[id] ?? 0;
    const button = (
      <button
        key={id}
        type="button"
        className="nav-item"
        aria-current={active ? "page" : undefined}
        aria-label={name}
        aria-keyshortcuts={shortcut ? `Control+${shortcut}` : undefined}
        aria-describedby={count > 0 ? `nav-badge-${id}` : undefined}
        onClick={() => setModule(id)}
      >
        <I className="nav-icon" aria-hidden="true" />
        {!compact && <span className="nav-label">{name}</span>}
        {count > 0 && (
          <span className="nav-badge">
            <span aria-hidden="true">{fmtInt(count)}</span>
            <span id={`nav-badge-${id}`} className="visually-hidden">
              {badgeText(id, count)}
            </span>
          </span>
        )}
      </button>
    );
    if (!compact) return button;
    const keys = shortcut ? ` · ${MOD}+${shortcut}` : "";
    return (
      <Tooltip key={id} side="right" content={`${name}${keys}`}>
        {button}
      </Tooltip>
    );
  };

  const toggleLabel = t(compact ? "nav.expand" : "nav.collapse");
  return (
    <nav
      aria-label={t("modules.navLabel")}
      className="module-nav"
      data-compact={compact || undefined}
    >
      {NAV_GROUPS.map((g) => {
        const items = MODULES.filter((m) => m.group === g.id);
        if (g.id === "bottom")
          return (
            <div key={g.id} className="nav-group nav-bottom">
              {items.map(item)}
              {!narrow && (
                <Tooltip side="right" content={`${toggleLabel} · ${MOD}+B`}>
                  <button
                    type="button"
                    className="nav-item nav-toggle"
                    aria-label={toggleLabel}
                    aria-keyshortcuts="Control+B"
                    onClick={toggleNav}
                  >
                    {compact ? (
                      <Icon.ChevronRight className="nav-icon" aria-hidden="true" />
                    ) : (
                      <Icon.ChevronLeft className="nav-icon" aria-hidden="true" />
                    )}
                  </button>
                </Tooltip>
              )}
            </div>
          );
        const label = t(`nav.groups.${g.id}`);
        return (
          <div key={g.id} role="group" aria-label={label} className="nav-group">
            {!compact && (
              <div className="nav-group-label" aria-hidden="true">
                {label}
              </div>
            )}
            {items.map(item)}
          </div>
        );
      })}
    </nav>
  );
}
