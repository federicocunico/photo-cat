// Vista "per codice": migliaia di errori uguali diventano una riga ("DECODE_FAILED — 1.240 file"),
// apribile con l'elenco paginato dei file e il motivo di ciascuno.

import { useInfiniteQuery, useQuery } from "@tanstack/react-query";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import {
  commands,
  isTauri,
  type ErrorEntry,
  type ErrorGroup,
  type ErrorQuery,
} from "../../lib/ipc";
import { fmtInt } from "../../lib/format";

/** File per pagina nell'elenco di un gruppo. */
export const GROUP_PAGE = 50;

const CAUSES = ["truncated", "corrupt", "unsupported_variant", "no_raw_preview", "too_large", "io"];

/** Motivo leggibile di un errore di decodifica (chiave all'inizio del dettaglio), se presente. */
export function useCauseText() {
  const { t } = useTranslation();
  return (detail: string | null) => {
    const key = detail?.split(":")[0] ?? "";
    return CAUSES.includes(key) ? t(`diag.causes.${key}`) : null;
  };
}

const groupKey = (g: ErrorGroup) => `${g.jobId}|${g.phase}|${g.code}|${g.level}`;

function GroupFiles({ group }: { group: ErrorGroup }) {
  const { t } = useTranslation();
  const cause = useCauseText();
  const files = useInfiniteQuery({
    queryKey: ["error-group-files", groupKey(group)],
    queryFn: ({ pageParam }) =>
      commands.queryErrors({
        level: group.level,
        phase: group.phase,
        jobId: group.jobId,
        code: group.code,
        text: null,
        limit: GROUP_PAGE,
        offset: pageParam,
      }),
    initialPageParam: 0,
    getNextPageParam: (last, all) =>
      last.length === GROUP_PAGE ? all.length * GROUP_PAGE : undefined,
    enabled: isTauri(),
  });
  const rows: ErrorEntry[] = files.data?.pages.flat() ?? [];
  return (
    <div style={{ padding: "4px 12px 10px 28px" }}>
      <ul
        aria-label={t("diag.groupFiles")}
        style={{ margin: 0, padding: 0, listStyle: "none", fontSize: 12 }}
      >
        {rows.map((r) => (
          <li
            key={r.id}
            style={{ display: "flex", gap: 10, padding: "2px 0" }}
            data-testid={`group-file-${r.id}`}
          >
            <span className="mono" style={{ wordBreak: "break-all", flex: 1 }}>
              {r.path ?? "—"}
            </span>
            <span className="muted">{cause(r.detail) ?? r.detail ?? r.message}</span>
          </li>
        ))}
      </ul>
      {files.hasNextPage && (
        <button
          type="button"
          className="btn btn-ghost"
          disabled={files.isFetchingNextPage}
          onClick={() => files.fetchNextPage()}
        >
          {t("diag.moreFiles", { n: fmtInt(Math.max(0, group.count - rows.length)) })}
        </button>
      )}
    </div>
  );
}

export function ErrorGroups({ query }: { query: ErrorQuery }) {
  const { t } = useTranslation();
  const [open, setOpen] = useState<string | null>(null);
  const groups = useQuery({
    queryKey: ["error-groups", { ...query, text: null, limit: 200, offset: 0 }],
    queryFn: () => commands.errorGroups({ ...query, text: null, limit: 200, offset: 0 }),
    enabled: isTauri(),
  });
  const list = groups.data ?? [];
  if (list.length === 0) return null;
  return (
    <section
      aria-label={t("diag.byCode")}
      style={{
        background: "var(--pc-panel)",
        border: "1px solid var(--pc-border)",
        borderRadius: 8,
        padding: "8px 0",
      }}
    >
      <h2 className="section-title" style={{ margin: "0 12px 6px" }}>
        {t("diag.byCode")}
      </h2>
      {list.map((g) => {
        const key = groupKey(g);
        const expanded = open === key;
        return (
          <div key={key}>
            <button
              type="button"
              aria-expanded={expanded}
              data-testid={`group-${g.code ?? "none"}-${g.phase ?? "none"}`}
              onClick={() => setOpen(expanded ? null : key)}
              style={{
                display: "flex",
                width: "100%",
                gap: 12,
                alignItems: "baseline",
                padding: "6px 12px",
                border: 0,
                background: expanded ? "var(--pc-raised)" : "transparent",
                color: "inherit",
                textAlign: "left",
                cursor: "pointer",
              }}
            >
              <strong className="mono">
                {t("diag.groupTitle", { code: g.code ?? "—", n: fmtInt(g.files || g.count) })}
              </strong>
              <span className="muted">
                {g.phase ? t(`jobs.phases.${g.phase}`, { defaultValue: g.phase }) : "—"}
                {g.jobId != null && ` · ${t("diag.job")} ${g.jobId}`}
              </span>
              <span className="muted" style={{ marginLeft: "auto", fontSize: 12 }}>
                {g.level}
              </span>
            </button>
            {expanded && <GroupFiles group={g} />}
          </div>
        );
      })}
    </section>
  );
}
