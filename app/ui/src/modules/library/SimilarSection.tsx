// Sezione "Foto simili" dell'Inspector: le altre foto del gruppo, la principale e le correzioni.

import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { commands, isTauri } from "../../lib/ipc";
import { useLibrary } from "../../store/library";
import { thumbUrl } from "./thumbUrl";

/** Viste da aggiornare dopo una correzione dei gruppi. */
export const refreshSimilar = (qc: ReturnType<typeof useQueryClient>) =>
  Promise.all([
    qc.invalidateQueries({ queryKey: ["similar-group"] }),
    qc.invalidateQueries({ queryKey: ["library"] }),
    qc.invalidateQueries({ queryKey: ["library-grid-count"] }),
  ]);

export function SimilarSection({ id }: { id: number }) {
  const { t } = useTranslation();
  const qc = useQueryClient();
  const setSelection = useLibrary((s) => s.setSelection);
  const [error, setError] = useState("");
  const group = useQuery({
    queryKey: ["similar-group", id],
    queryFn: () => commands.similarGroup(id),
    enabled: isTauri(),
  });
  const g = group.data;
  if (!g) return null;
  const run = async (fn: () => Promise<unknown>) => {
    setError("");
    try {
      await fn();
      await refreshSimilar(qc);
    } catch (e) {
      setError((e as { detail?: string }).detail ?? String(e));
    }
  };
  const isCover = g.cover === id;
  return (
    <section aria-label={t("similar.title")} data-testid="similar-section">
      <h3 className="section-title">
        {t("similar.title")} ({g.members.length})
      </h3>
      {isCover && <p className="muted">{t("similar.isCover")}</p>}
      <div style={{ display: "flex", flexWrap: "wrap", gap: 6 }}>
        {g.members.map((m) => {
          const url = thumbUrl(m.thumb);
          return (
            <button
              key={m.id}
              type="button"
              title={m.name}
              data-testid={`similar-member-${m.id}`}
              aria-current={m.id === id}
              onClick={() => setSelection([m.id], m.id)}
              style={{
                width: 56,
                height: 42,
                padding: 0,
                borderRadius: 4,
                overflow: "hidden",
                cursor: "pointer",
                border: `2px solid ${
                  m.id === g.cover
                    ? "var(--pc-warn)"
                    : m.id === id
                      ? "var(--pc-accent)"
                      : "transparent"
                }`,
                background: "var(--pc-thumb-bg)",
              }}
            >
              {url && (
                <img
                  src={url}
                  alt=""
                  style={{ width: "100%", height: "100%", objectFit: "cover" }}
                />
              )}
            </button>
          );
        })}
      </div>
      <div className="feature-actions" style={{ marginTop: 8 }}>
        {!isCover && (
          <button
            type="button"
            className="btn"
            onClick={() => run(() => commands.similarSetCover(g.id, id))}
          >
            {t("similar.setCover")}
          </button>
        )}
        <button
          type="button"
          className="btn"
          onClick={() => run(() => commands.similarRemove(id))}
        >
          {t("similar.remove")}
        </button>
        <button
          type="button"
          className="btn btn-ghost"
          onClick={() => run(() => commands.similarDissolve(g.id))}
        >
          {t("similar.dissolve")}
        </button>
      </div>
      {error && <p role="alert">{error}</p>}
    </section>
  );
}
