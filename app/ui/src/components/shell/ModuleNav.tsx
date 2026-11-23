import { Coachmark } from "../guide/Coachmark";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { useQuery } from "@tanstack/react-query";
import { MODULES, NAV_GROUPS, type NavItem } from "../../modules/registry";
import { useLayout, type ModuleId } from "../../store/layout";
import { activeJobs, useLive } from "../../store/live";
import { commands } from "../../lib/ipc";
import { useCommands, type Command } from "../../commands/registry";
import { fmtInt } from "../../lib/format";
import { Icon } from "../icons";
import { Tooltip, chooseFolder } from "../ui";
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

/** Comandi della navigazione: Ctrl+1…9 per le voci dei gruppi, Ctrl+, Impostazioni, Ctrl+B. */
function useNavCommands() {
  const { t } = useTranslation();
  const openModule = useLayout((s) => s.openModule);
  useCommands([
    ...MODULES.map((m, i): Command => ({
      id: `nav.${m.id}`,
      // Stesso nome nella palette e nel menu Vai (application-menu).
      label: t(`modules.${m.id}`),
      group: "navigation",
      keywords: t("commands.goTo", { name: t(`modules.${m.id}`) }),
      menu: [
        { menu: "go" as const, section: m.group === "bottom" ? 1 : 0, order: i },
        ...(m.id === "settings" ? [{ menu: "file" as const, section: 2, order: 1 }] : []),
      ],
      shortcut: m.shortcut
        ? { key: String(m.shortcut), mod: true }
        : m.id === "settings"
          ? { key: ",", mod: true }
          : undefined,
      run: () => openModule({ module: m.id }),
    })),
    {
      id: "nav.raw-review.open",
      label: t("menu.openRawFolder"),
      group: "navigation",
      menu: { menu: "file", section: 1, order: 0 },
      keywords: `${t("modules.raw-review")} RAW Fujifilm X-T5 FujiView`,
      // Apre il dialogo di scelta cartella e porta la cartella scelta nel modulo.
      run: async () => {
        const root = await chooseFolder().catch(() => null);
        openModule(root ? { module: "raw-review", root } : { module: "raw-review" });
      },
    },
    ...(["overview", "reprocess", "cleanup", "map"] as const).map((section): Command => ({
      id: `nav.ai.${section}`,
      label:
        section === "map"
          ? t("menu.photoMap")
          : section === "reprocess"
            ? t("menu.reanalyze")
            : t(`ai.commands.${section}`),
      group: "navigation",
      keywords: t("modules.ai"),
      menu:
        section === "map"
          ? { menu: "view", section: 2, order: 0 }
          : section === "reprocess"
            ? { menu: "tools", section: 0, order: 0 }
            : undefined,
      run: () => openModule({ module: "ai", section }),
    })),
    {
      id: "nav.toggle",
      label: t("menu.compactNav"),
      group: "navigation",
      shortcut: { key: "b", mod: true },
      menu: { menu: "view", section: 0, order: 0 },
      checked: () => useLayout.getState().navCompact,
      run: () => useLayout.getState().toggleNav(),
    },
  ]);
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
  const openModule = useLayout((s) => s.openModule);
  const preferCompact = useLayout((s) => s.navCompact);
  const toggleNav = useLayout((s) => s.toggleNav);
  const narrow = useNarrowWindow();
  const compact = preferCompact || narrow;
  const badges = useBadges();
  useNavCommands();

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
        onClick={() => openModule({ module: id })}
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
              <Coachmark tip="moved" learnMore="settings" />
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
