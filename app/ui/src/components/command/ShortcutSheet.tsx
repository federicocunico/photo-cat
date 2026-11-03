import { useTranslation } from "react-i18next";
import { Dialog } from "../ui";
import { SHEET_GROUPS } from "../../commands/catalog";
import { useCommandStore } from "../../commands/registry";
import { useShortcutSheet } from "../../store/shortcutSheet";

/** Tutte le scorciatoie dell'app, per area, nella lingua dell'interfaccia. */
export function ShortcutSheet() {
  const { t } = useTranslation();
  const open = useShortcutSheet((s) => s.open);
  const setOpen = useShortcutSheet((s) => s.setOpen);
  const docs = useCommandStore((s) => s.commands).filter((c) => c.id.startsWith("doc."));
  return (
    <Dialog open={open} onOpenChange={setOpen} title={t("shortcuts.title")} width={720}>
      <div className="shortcut-sheet">
        {SHEET_GROUPS.map((group) => {
          const items = docs.filter((c) => c.group === group);
          if (!items.length) return null;
          return (
            <section key={group} aria-labelledby={`sheet-${group}`}>
              <h3 id={`sheet-${group}`}>{t(`commands.groups.${group}`)}</h3>
              <dl>
                {items.map((c) => (
                  <div key={c.id} className="shortcut-row">
                    <dt>
                      <kbd className="ui-kbd">{c.keys}</kbd>
                    </dt>
                    <dd>{c.label}</dd>
                  </div>
                ))}
              </dl>
            </section>
          );
        })}
      </div>
    </Dialog>
  );
}
