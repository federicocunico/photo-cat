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
  const setModule = useLayout((s) => s.setModule);
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
    ["sources.openImport", () => setModule("import")],
    ...(missing > 0
      ? ([
          ["sources.relink", () => setDialog("relink")],
          ["sources.removeMissing", () => setDialog("missing")],
        ] as Array<[string, () => unknown]>)
      : []),
    ["sources.remove", () => setDialog("remove")],
  ];
  return (
    <div ref={ref} style={{ position: "relative" }}>
      <button
        type="button"
        className="btn btn-ghost"
        style={{ padding: "0 6px", fontSize: 13 }}
        aria-haspopup="menu"
        aria-expanded={open}
        aria-label={t("sources.menu", { name: label })}
        onClick={() => setOpen(!open)}
      >
        …
      </button>
      {open && (
        <div
          role="menu"
          aria-label={t("sources.menu", { name: label })}
          onKeyDown={(e) => e.key === "Escape" && setOpen(false)}
          style={{
            position: "absolute",
            right: 0,
            top: "100%",
            zIndex: 20,
            minWidth: 210,
            display: "flex",
            flexDirection: "column",
            padding: 4,
            border: "1px solid var(--pc-border)",
            borderRadius: 6,
            background: "var(--pc-raised)",
            boxShadow: "0 4px 16px rgba(0,0,0,0.18)",
          }}
        >
          {items.map(([key, f]) => (
            <button
              key={key}
              type="button"
              role="menuitem"
              className="btn btn-ghost"
              style={{
                justifyContent: "flex-start",
                textAlign: "left",
                color: key === "sources.remove" ? "var(--pc-danger)" : undefined,
              }}
              onClick={act(f)}
            >
              {t(key)}
            </button>
          ))}
        </div>
      )}
      {error && (
        <span role="alert" style={{ fontSize: 12, color: "var(--pc-danger)" }}>
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
