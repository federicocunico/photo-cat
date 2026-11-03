import { useState } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { commands, type ImportPreview } from "../../lib/ipc";
import { Button, InlineAlert } from "../../components/ui";
import "./people.css";

const errorText = (e: unknown) => (e as { detail?: string }).detail ?? String(e);
const KEY = ["identities"] as const;

/** Esporta / importa identità (JSON), riusabile dalla scheda Persone. */
export function Transfer({ onDone }: { onDone: () => void }) {
  const { t } = useTranslation();
  const qc = useQueryClient();
  const [includeFaces, setIncludeFaces] = useState(false);
  const [preview, setPreview] = useState<{ path: string; p: ImportPreview } | null>(null);
  const [merge, setMerge] = useState<string[]>([]);
  const [message, setMessage] = useState("");
  const [error, setError] = useState("");
  const run = async (fn: () => Promise<unknown>) => {
    setError("");
    try {
      await fn();
    } catch (e) {
      setError(errorText(e));
    }
  };
  const exportFile = () =>
    run(async () => {
      const { save } = await import("@tauri-apps/plugin-dialog");
      const path = await save({
        defaultPath: "photo-cat-identita.json",
        filters: [{ name: "JSON", extensions: ["json"] }],
      });
      if (!path) return;
      await commands.exportIdentities(path, includeFaces);
      setMessage(t("identities.exported"));
    });
  const importFile = () =>
    run(async () => {
      const { open } = await import("@tauri-apps/plugin-dialog");
      const path = await open({
        multiple: false,
        filters: [{ name: "JSON", extensions: ["json"] }],
      });
      if (typeof path !== "string") return;
      const p = await commands.previewIdentityImport(path);
      setPreview({ path, p });
      setMerge(p.conflicts.map((c) => c.name));
    });
  const apply = () =>
    run(async () => {
      if (!preview) return;
      const out = await commands.importIdentities(preview.path, merge);
      setPreview(null);
      setMessage(
        t("identities.imported", {
          created: out.created,
          merged: out.merged,
          skipped: out.skipped,
        }),
      );
      await qc.invalidateQueries({ queryKey: KEY });
      onDone();
    });
  return (
    <div className="people-stack people-stack--gap">
      <div className="feature-actions people-actions-flush">
        <Button onClick={() => importFile()}>{t("identities.import")}</Button>
        <Button onClick={() => exportFile()}>{t("identities.export")}</Button>
        <label className="people-check">
          <input
            type="checkbox"
            checked={includeFaces}
            onChange={(e) => setIncludeFaces(e.target.checked)}
          />
          {t("identities.includeFaces")}
        </label>
      </div>
      {preview && (
        <div
          className="feature-card people-card-stretch"
          role="group"
          aria-label={t("identities.importPreview")}
        >
          <strong>{t("identities.importPreview")}</strong>
          <span>{t("identities.importNew", { count: preview.p.new.length })}</span>
          {preview.p.conflicts.map((c) => (
            <label key={c.name} className="people-check">
              <input
                type="checkbox"
                checked={merge.includes(c.name)}
                onChange={(e) =>
                  setMerge((m) =>
                    e.target.checked ? [...m, c.name] : m.filter((x) => x !== c.name),
                  )
                }
              />
              {t("identities.importMerge", { name: c.name })}
            </label>
          ))}
          <div className="feature-actions people-actions-flush">
            <Button variant="primary" onClick={() => apply()}>
              {t("identities.importApply")}
            </Button>
            <Button onClick={() => setPreview(null)}>{t("identities.cancel")}</Button>
          </div>
        </div>
      )}
      {message && <p role="status">{message}</p>}
      {error && (
        <InlineAlert tone="danger" title={t("people.failed")}>
          {error}
        </InlineAlert>
      )}
    </div>
  );
}
