import { useId, useState } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { commands, isTauri, type IdentitySuggestion } from "../../lib/ipc";

const errorText = (e: unknown) => (e as { detail?: string }).detail ?? String(e);

/**
 * Campo "Chi è?" di un gruppo di volti: propone le identità dell'app il cui
 * nome o alias contiene il testo, ordinate per somiglianza, e crea l'identità
 * se non esiste. Collegare a un'identità già collegata unisce i gruppi.
 */
export function WhoIs({
  person,
  linked,
  onLinked,
}: {
  person: number;
  /** Nome dell'identità collegata, se presente. */
  linked: string | null;
  onLinked: (person: number) => void;
}) {
  const { t } = useTranslation();
  const qc = useQueryClient();
  const listId = useId();
  const [text, setText] = useState("");
  const [open, setOpen] = useState(false);
  const [active, setActive] = useState(0);
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);
  const query = text.trim();
  const suggestions = useQuery({
    queryKey: ["identity-suggestions", person, query],
    queryFn: () => commands.suggestIdentities(person, query),
    enabled: isTauri() && open && query.length > 0,
  });
  const items: IdentitySuggestion[] = suggestions.data ?? [];
  const exact = items.some((s) => s.name.toLowerCase() === query.toLowerCase());
  const options: Array<{ kind: "link"; s: IdentitySuggestion } | { kind: "create" }> = [
    ...items.map((s) => ({ kind: "link" as const, s })),
    ...(query && suggestions.isSuccess && !exact ? [{ kind: "create" as const }] : []),
  ];
  const done = async (p: number) => {
    setText("");
    setOpen(false);
    await Promise.all([
      qc.invalidateQueries({ queryKey: ["people"] }),
      qc.invalidateQueries({ queryKey: ["faces"] }),
      qc.invalidateQueries({ queryKey: ["identities"] }),
    ]);
    onLinked(p);
  };
  const run = async (fn: () => Promise<number>) => {
    setBusy(true);
    setError("");
    try {
      await done(await fn());
    } catch (e) {
      setError(errorText(e));
    } finally {
      setBusy(false);
    }
  };
  const choose = (i: number) => {
    const o = options[i];
    if (!o) return;
    if (o.kind === "link") void run(() => commands.linkPersonIdentity(person, o.s.uid));
    else
      void run(async () => {
        const uid = await commands.saveIdentity(null, {
          name: query,
          shortName: null,
          group: null,
          threshold: null,
          aliases: [],
        });
        return commands.linkPersonIdentity(person, uid);
      });
  };
  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 6, position: "relative" }}>
      <div className="feature-actions" style={{ margin: 0 }}>
        <input
          className="field"
          role="combobox"
          aria-label={t("identities.whoIs")}
          aria-expanded={open && options.length > 0}
          aria-controls={listId}
          aria-autocomplete="list"
          placeholder={linked ?? t("identities.whoIsPlaceholder")}
          value={text}
          disabled={busy}
          onChange={(e) => {
            setText(e.target.value);
            setOpen(true);
            setActive(0);
          }}
          onKeyDown={(e) => {
            if (e.key === "ArrowDown") {
              e.preventDefault();
              setActive((a) => Math.min(a + 1, options.length - 1));
            } else if (e.key === "ArrowUp") {
              e.preventDefault();
              setActive((a) => Math.max(a - 1, 0));
            } else if (e.key === "Enter" && options.length) {
              e.preventDefault();
              choose(active);
            } else if (e.key === "Escape") setOpen(false);
          }}
          style={{ minWidth: 260 }}
        />
        {linked && (
          <button
            className="btn"
            disabled={busy}
            onClick={() => void run(() => commands.linkPersonIdentity(person, null))}
          >
            {t("identities.unlink")}
          </button>
        )}
      </div>
      {open && options.length > 0 && (
        <ul
          id={listId}
          role="listbox"
          aria-label={t("identities.suggestions")}
          style={{
            listStyle: "none",
            margin: 0,
            padding: 4,
            background: "var(--pc-panel)",
            border: "1px solid var(--pc-border)",
            borderRadius: 6,
            maxWidth: 420,
          }}
        >
          {options.map((o, i) => (
            <li
              key={o.kind === "link" ? o.s.uid : "create"}
              role="option"
              aria-selected={i === active}
              onMouseDown={(e) => {
                e.preventDefault();
                choose(i);
              }}
              style={{
                padding: "6px 10px",
                borderRadius: 4,
                cursor: "pointer",
                background: i === active ? "var(--pc-selection-bg)" : undefined,
                display: "flex",
                justifyContent: "space-between",
                gap: 12,
              }}
            >
              {o.kind === "link" ? (
                <>
                  <span>{o.s.name}</span>
                  <span className="muted mono" style={{ fontSize: 12 }}>
                    {o.s.score !== null ? o.s.score.toFixed(2) : ""}
                  </span>
                </>
              ) : (
                <span>{t("identities.createNamed", { name: query })}</span>
              )}
            </li>
          ))}
        </ul>
      )}
      {error && <p role="alert">{error}</p>}
    </div>
  );
}
