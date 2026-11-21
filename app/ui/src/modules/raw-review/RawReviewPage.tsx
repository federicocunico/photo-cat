// Modulo "Varianti RAW (Fujifilm X-T5)": apertura di una cartella o di un progetto FujiView,
// sessioni recenti e, a cartella aperta, il riepilogo della scansione. Il confronto delle
// varianti arriva con i task 3.3 e 3.4.
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Icon } from "../../components/icons";
import {
  Button,
  Card,
  EmptyState,
  IconButton,
  InlineAlert,
  PageHeader,
  chooseFolder,
} from "../../components/ui";
import { commands, type OpenedReview } from "../../lib/ipc";
import { fmtDate } from "../../lib/format";
import { useLayout } from "../../store/layout";
import { notifyError } from "../../store/toasts";
import { chooseProject } from "./pickers";
import "./raw-review.css";

type OpenFailure = { kind: "unsupported" | "failed"; detail: string };

const RECENT_KEY = ["raw-review-recent"] as const;

/** Un errore dei comandi: `INVALID_INPUT` all'apertura di una cartella è "forma non supportata". */
function failure(e: unknown, folder: boolean): OpenFailure {
  const err = e as { code?: string; detail?: string };
  return {
    kind: folder && err?.code === "INVALID_INPUT" ? "unsupported" : "failed",
    detail: err?.detail ?? String(e),
  };
}

export function RawReviewPage() {
  const { t } = useTranslation();
  const qc = useQueryClient();
  const target = useLayout((s) => s.rawReviewTarget);
  const clearTarget = useLayout((s) => s.clearRawReviewTarget);
  const [opened, setOpened] = useState<OpenedReview | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<OpenFailure | null>(null);

  const recent = useQuery({
    queryKey: RECENT_KEY,
    queryFn: () => commands.rawReviewRecent(),
  });

  const run = useCallback(
    async (open: () => Promise<OpenedReview>, folder: boolean) => {
      setBusy(true);
      setError(null);
      try {
        setOpened(await open());
      } catch (e) {
        setError(failure(e, folder));
      } finally {
        setBusy(false);
        void qc.invalidateQueries({ queryKey: RECENT_KEY });
      }
    },
    [qc],
  );

  const openFolder = useCallback(
    (root: string) => run(() => commands.rawReviewOpen(root), true),
    [run],
  );

  // Destinazione dal resto dell'app (comando di palette, collegamenti): si consuma una volta.
  useEffect(() => {
    if (!target) return;
    clearTarget();
    if (target.root) void openFolder(target.root);
  }, [target, clearTarget, openFolder]);

  const pickFolder = async () => {
    const root = await chooseFolder().catch(() => null);
    if (root) await openFolder(root);
  };

  const pickProject = async () => {
    const file = await chooseProject().catch(() => null);
    if (file) await run(() => commands.rawReviewOpenFujiview(file), false);
  };

  const forget = async (root: string) => {
    try {
      await commands.rawReviewForget(root);
      await qc.invalidateQueries({ queryKey: RECENT_KEY });
    } catch {
      notifyError(null, t("rawReviewPage.forgetFailed"));
    }
  };

  const close = async () => {
    if (!opened) return;
    const { sessionId } = opened;
    setOpened(null);
    try {
      await commands.rawReviewSave(sessionId);
      await commands.rawReviewClose(sessionId);
    } catch {
      // La sessione è già chiusa o non salvabile: la prossima apertura riparte dai dati salvati.
    }
    void qc.invalidateQueries({ queryKey: RECENT_KEY });
  };

  const actions = (
    <>
      <Button icon={<Icon.Import size={16} />} busy={busy} onClick={pickFolder}>
        {t("rawReviewPage.openFolder")}
      </Button>
      <Button icon={<Icon.Import size={16} />} busy={busy} onClick={pickProject}>
        {t("rawReviewPage.openProject")}
      </Button>
    </>
  );

  return (
    <div className="module-placeholder raw-review-page">
      <PageHeader
        title={t("modules.raw-review")}
        description={t("rawReviewPage.subtitle")}
        actions={actions}
      />
      {error && (
        <InlineAlert
          tone="danger"
          title={t(
            error.kind === "unsupported"
              ? "rawReviewPage.unsupportedTitle"
              : "rawReviewPage.openFailedTitle",
          )}
          onDismiss={() => setError(null)}
        >
          {error.kind === "unsupported" ? t("rawReviewPage.unsupportedBody") : error.detail}
        </InlineAlert>
      )}
      {opened ? (
        <OpenedSummary opened={opened} onClose={close} />
      ) : (
        <div className="raw-review-start">
          <EmptyState
            icon={<Icon.RawReview size={32} aria-hidden="true" />}
            title={t("rawReviewPage.emptyTitle")}
            description={t("rawReviewPage.emptyHint")}
          />
          <Card title={t("rawReviewPage.recentTitle")}>
            {recent.data && recent.data.length > 0 ? (
              <ul className="raw-review-recent">
                {recent.data.map((r) => (
                  <li key={r.sourceRoot} className="raw-review-recent-item">
                    <button
                      type="button"
                      className="raw-review-recent-open"
                      disabled={busy}
                      onClick={() => void openFolder(r.sourceRoot)}
                    >
                      <strong>{r.name}</strong>
                      <span className="muted raw-review-recent-root">{r.sourceRoot}</span>
                      <span className="muted">
                        {t("rawReviewPage.recentMeta", {
                          shots: r.shotCount,
                          choices: r.choiceCount,
                          date: fmtDate(r.lastUsedAt),
                        })}
                      </span>
                    </button>
                    <IconButton
                      label={t("rawReviewPage.recentForgetLabel", { name: r.name })}
                      icon={<Icon.Close size={16} />}
                      onClick={() => forget(r.sourceRoot)}
                    />
                  </li>
                ))}
              </ul>
            ) : (
              <p className="muted">{t("rawReviewPage.recentEmpty")}</p>
            )}
          </Card>
        </div>
      )}
    </div>
  );
}

/** Riepilogo della cartella aperta, con il segnaposto del confronto (task 3.3). */
function OpenedSummary({ opened, onClose }: { opened: OpenedReview; onClose: () => void }) {
  const { t } = useTranslation();
  return (
    <Card
      title={opened.session.name}
      actions={
        <Button variant="ghost" onClick={onClose}>
          {t("rawReviewPage.closeSession")}
        </Button>
      }
    >
      <dl className="raw-review-facts">
        <dt>{t("rawReviewPage.layoutLabel")}</dt>
        <dd>{t(`rawReviewPage.layout.${opened.layout}`)}</dd>
        <dt>{t("rawReviewPage.shotsLabel")}</dt>
        <dd>{t("rawReviewPage.shotCount", { count: opened.shots.length })}</dd>
      </dl>
      {opened.warnings.length > 0 && (
        <InlineAlert tone="warn" title={t("rawReviewPage.warningsTitle")}>
          <ul className="raw-review-warnings">
            {opened.warnings.map((w) => (
              <li key={w}>{w}</li>
            ))}
          </ul>
        </InlineAlert>
      )}
      <div className="raw-review-compare muted">{t("rawReviewPage.compareSoon")}</div>
    </Card>
  );
}
