// Riepilogo: migliaia di errori uguali diventano una riga ("Impossibile leggere l'immagine —
// 1.240 file"), apribile con l'elenco paginato dei file e il motivo di ciascuno.

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
import { Badge, Button } from "../../components/ui";
import { Icon } from "../../components/icons";
import { GROUP_PAGE, useCauseText, useEntryLabels, useErrorText } from "./errors";

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
    <div className="diag-group-files">
      <ul aria-label={t("diag.groupFiles")}>
        {rows.map((r) => (
          <li key={r.id} data-testid={`group-file-${r.id}`}>
            <span className="mono diag-path">{r.path ?? "—"}</span>
            <span className="muted">{cause(r.detail) ?? r.detail ?? r.message}</span>
          </li>
        ))}
      </ul>
      {files.hasNextPage && (
        <Button
          variant="ghost"
          size="sm"
          disabled={files.isFetchingNextPage}
          onClick={() => files.fetchNextPage()}
        >
          {t("diag.moreFiles", { n: fmtInt(Math.max(0, group.count - rows.length)) })}
        </Button>
      )}
    </div>
  );
}

export function ErrorGroups({ query }: { query: ErrorQuery }) {
  const { t } = useTranslation();
  const text = useErrorText();
  const labels = useEntryLabels();
  const [open, setOpen] = useState<string | null>(null);
  const groups = useQuery({
    queryKey: ["error-groups", { ...query, text: null, limit: 200, offset: 0 }],
    queryFn: () => commands.errorGroups({ ...query, text: null, limit: 200, offset: 0 }),
    enabled: isTauri(),
  });
  const list = groups.data ?? [];
  if (list.length === 0) return null;
  return (
    <section aria-labelledby="diag-by-code" className="diag-groups">
      <h3 id="diag-by-code" className="section-title">
        {t("diag.byCode")}
      </h3>
      {list.map((g) => {
        const key = groupKey(g);
        const expanded = open === key;
        const message = text({ code: g.code, message: g.message }).replace(/\.$/, "");
        return (
          <div key={key}>
            <button
              type="button"
              className="diag-group"
              aria-expanded={expanded}
              data-testid={`group-${g.code ?? "none"}-${g.phase ?? "none"}`}
              onClick={() => setOpen(expanded ? null : key)}
            >
              {expanded ? <Icon.ChevronDown size={14} /> : <Icon.ChevronRight size={14} />}
              <strong>{t("diag.groupTitle", { message, n: fmtInt(g.files || g.count) })}</strong>
              <span className="muted">
                {labels.phase(g.phase)}
                {g.jobId != null && ` · ${t("diag.job")} ${g.jobId}`}
              </span>
              <span className="diag-group-level">
                <Badge tone={g.level === "ERROR" ? "danger" : "warn"}>
                  {labels.level(g.level)}
                </Badge>
              </span>
            </button>
            {expanded && <GroupFiles group={g} />}
          </div>
        );
      })}
    </section>
  );
}
