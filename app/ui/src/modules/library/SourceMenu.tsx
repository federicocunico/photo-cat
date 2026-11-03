import { BusyButton } from "../../components/feedback/BusyButton";
import { Icon } from "../../components/icons";
// Menu "…" di una sorgente nella barra laterale della Libreria: aggiornare,
// aggiornare una cartella, rimuovere, aprire in Import e gestire i mancanti.
import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import type { SourceSummary } from "../../lib/ipc";
import { useLayout } from "../../store/layout";
import { commands } from "../../lib/ipc";
import { RelinkDialog, RemoveMissingDialog, RemoveSourceDialog } from "../import/manage";
import { updateFolder } from "../import/sourceActions";

type Dialog = "remove" | "relink" | "missing" | null;

export function SourceMenu({ summary, label }: { summary: SourceSummary; label: string }) {
  const { t } = useTranslation();
  const openModule = useLayout((s) => s.openModule);
  const { source, missing } = summary;
  const [open, setOpen] = useState(false);
  const [dialog, setDialog] = useState<Dialog>(null);
  const [error, setError] = useState("");
  const ref = useRef<HTMLDivElement>(null);
  useEffect(() => {
    if (!open) return;
    const close = (e: MouseEvent) => {
      if (!ref.current?.contains(e.target as Node)) setOpen(false);
    };
    document.addEventListener("mousedown", close);
    return () => document.removeEventListener("mousedown", close);
  }, [open]);
  const act = (f: () => unknown) => async () => {
    setOpen(false);
    setError("");
    try {
      await f();
    } catch (e) {
      setError((e as { detail?: string }).detail ?? String(e));
    }
  };
  const items: Array<[string, () => unknown]> = [
    ["sources.update", () => commands.startImport(source.id, null)],
    ["sources.updateFolder", () => updateFolder(source)],
    ["sources.openImport", () => openModule({ module: "import" })],
    ...(missing > 0
      ? ([
          ["sources.relink", () => setDialog("relink")],
          ["sources.removeMissing", () => setDialog("missing")],
        ] as Array<[string, () => unknown]>)
      : []),
    ["sources.remove", () => setDialog("remove")],
  ];
  return (
    <div ref={ref} className="source-menu">
      <BusyButton
        type="button"
        className="btn btn-ghost source-menu-trigger"
        aria-haspopup="menu"
        aria-expanded={open}
        aria-label={t("sources.menu", { name: label })}
        onClick={() => setOpen(!open)}
      >
        <Icon.More size={16} />
      </BusyButton>
      {open && (
        <div
          role="menu"
          aria-label={t("sources.menu", { name: label })}
          onKeyDown={(e) => e.key === "Escape" && setOpen(false)}
          className="source-menu-pop"
        >
          {items.map(([key, f]) => (
            <BusyButton
              key={key}
              type="button"
              role="menuitem"
              className="btn btn-ghost source-menu-item"
              data-danger={key === "sources.remove" || undefined}
              onClick={act(f)}
            >
              {t(key)}
            </BusyButton>
          ))}
        </div>
      )}
      {error && (
        <span role="alert" className="source-menu-error">
          {error}
        </span>
      )}
      {dialog === "remove" && (
        <RemoveSourceDialog source={source} onClose={() => setDialog(null)} />
      )}
      {dialog === "relink" && (
        <RelinkDialog source={source} missing={missing} onClose={() => setDialog(null)} />
      )}
      {dialog === "missing" && (
        <RemoveMissingDialog
          sourceId={source.id}
          missing={missing}
          onClose={() => setDialog(null)}
        />
      )}
    </div>
  );
}
