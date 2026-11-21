// Modulo "Varianti RAW (Fujifilm X-T5)": apertura di una cartella o di un progetto FujiView,
// sessioni recenti e, a cartella aperta, il confronto delle varianti. La sessione aperta sta in
// `useRawReview`: smontare il modulo non la chiude.
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
import { commands, type OpenedReview, type ShotDecision } from "../../lib/ipc";
import { fmtDate } from "../../lib/format";
import { useLayout } from "../../store/layout";
import {
  candidatesOf,
  chooseWinner,
  movePreset,
  saveOpenReview,
  selectShot,
  setRating,
  showAllPresets,
  toggleVisible,
  useRawReview,
} from "../../store/rawReview";
import { notifyError } from "../../store/toasts";
import { ComparePanels } from "./ComparePanels";
import { InfoSidebar } from "./InfoSidebar";
import { RatingRow } from "../library/InspectorQuality";
import { focusedPanel } from "./panelFocus";
import { presetLabel } from "./presetLabel";
import { chooseProject } from "./pickers";
import { ShotList } from "./ShotList";
import { useRawReviewShortcuts } from "./useRawReviewShortcuts";
import { forgetMetadata, useShotMetadata } from "./useShotMetadata";
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
  const opened = useRawReview((s) => s.opened);
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
        // Riaprire la stessa cartella riparte dai dati salvati: prima si salva la sessione attuale.
        await saveOpenReview();
        const next = await open();
        await leaveCurrent(next.sessionId);
        useRawReview.getState().setOpened(next);
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

  // Lasciando il modulo la sessione resta aperta nello store (Ctrl+9 la ritrova): si salvano
  // le scelte, anche quelle ancora nell'attesa del salvataggio automatico, così nulla va perso
  // cambiando modulo.
  useEffect(() => () => void saveOpenReview(), []);

  const close = async () => {
    if (!opened) return;
    useRawReview.getState().clear();
    await leaveCurrent(null, opened);
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
    <div className={`raw-review-page${opened ? " is-open" : ""}`}>
      {/* Con una cartella aperta l'intestazione perde la descrizione: l'altezza va ai pannelli. */}
      <PageHeader
        title={t("modules.raw-review")}
        description={opened ? undefined : t("rawReviewPage.subtitle")}
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
        <Review opened={opened} onClose={close} />
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

/**
 * Salva e chiude la sessione aperta nel backend, a meno che non sia quella appena aperta
 * (`keep`): riaprire la stessa cartella la sostituisce già da sola.
 */
async function leaveCurrent(keep: string | null, current?: OpenedReview): Promise<void> {
  const previous = current ?? useRawReview.getState().opened;
  if (!previous || previous.sessionId === keep) return;
  try {
    await commands.rawReviewSave(previous.sessionId);
    await commands.rawReviewClose(previous.sessionId);
  } catch {
    // La sessione è già chiusa o non salvabile: la prossima apertura riparte dai dati salvati.
  }
  forgetMetadata(previous.sessionId);
}

/** Barra della cartella aperta e confronto delle varianti. */
function Review({ opened, onClose }: { opened: OpenedReview; onClose: () => void }) {
  const { t } = useTranslation();
  const shotIndex = useRawReview((s) => s.shotIndex);
  const shot = opened.shots[shotIndex];
  const metadata = useShotMetadata(opened.sessionId, shot);
  const panels = opened.session.panelPresets;
  const candidates = candidatesOf(opened);
  const decision = shot ? opened.session.decisions?.[shot.key] : undefined;
  const decided = Object.values(opened.session.decisions ?? {}).filter(
    (d) => d.winnerPreset,
  ).length;
  const total = opened.shots.length;

  // Pannello a fuoco per Invio e frecce: quello scelto, altrimenti la variante migliore o il primo.
  // Un altro scatto riparte da capo.
  const [panelChoice, setPanelChoice] = useState<{
    shot: string | undefined;
    index: number | null;
  }>({ shot: shot?.key, index: null });
  const chosenPanel = panelChoice.shot === shot?.key ? panelChoice.index : null;
  const focused = focusedPanel(panels, chosenPanel, decision?.winnerPreset ?? null);
  const focusPanel = (index: number) => setPanelChoice({ shot: shot?.key, index });

  useRawReviewShortcuts({ shot, panels, focused, onFocus: focusPanel }, true);
  const announcement = useDecisionAnnouncement(shot?.key, decision);

  return (
    <div className="rr-review">
      <div className="rr-bar">
        <h2 className="rr-bar-name">{opened.session.name}</h2>
        <span className="muted">{t(`rawReviewPage.layout.${opened.layout}`)}</span>
        <span className="muted">{t("rawReviewPage.shotCount", { count: total })}</span>
        {opened.warnings.length > 0 && (
          <details className="rr-warnings">
            <summary>
              {t("rawReviewPage.warningsTitle")} ({opened.warnings.length})
            </summary>
            <ul className="raw-review-warnings">
              {opened.warnings.map((w) => (
                <li key={w}>{w}</li>
              ))}
            </ul>
          </details>
        )}
        <Button className="rr-bar-close" variant="ghost" onClick={onClose}>
          {t("rawReviewPage.closeSession")}
        </Button>
      </div>
      <div className="rr-workspace">
        <ShotList
          sessionId={opened.sessionId}
          shots={opened.shots}
          decisions={opened.session.decisions}
          current={shotIndex}
          onOpen={selectShot}
        />
        <div className="rr-main">
          {shot ? (
            <ComparePanels
              sessionId={opened.sessionId}
              shot={shot}
              panels={panels}
              winnerPreset={decision?.winnerPreset ?? null}
              focusedIndex={focused}
              onFocusPanel={focusPanel}
              metadata={metadata}
              onChoose={(preset) => void chooseWinner(shot.key, preset)}
              onHide={(preset) => void toggleVisible(preset)}
            />
          ) : (
            <p className="muted rr-info-empty">{t("rawReviewInfo.noShot")}</p>
          )}
          <nav className="rr-footer" aria-label={t("rawReviewCompare.shotNav")}>
            <IconButton
              label={t("rawReviewCompare.previousShot")}
              icon={<Icon.ChevronLeft size={18} />}
              disabled={shotIndex <= 0}
              onClick={() => selectShot(shotIndex - 1)}
            />
            <strong className="rr-footer-pos" aria-live="polite">
              {t("rawReviewCompare.position", {
                current: total ? shotIndex + 1 : 0,
                total,
                name: shot?.displayName ?? "",
              })}
            </strong>
            <IconButton
              label={t("rawReviewCompare.nextShot")}
              icon={<Icon.ChevronRight size={18} />}
              disabled={shotIndex >= total - 1}
              onClick={() => selectShot(shotIndex + 1)}
            />
            {shot && (
              <div className="rr-rating">
                <RatingRow
                  rating={decision?.rating ?? 0}
                  apply={async (operation) => {
                    if (operation.action === "set_rating")
                      void setRating(shot.key, operation.rating ?? 0);
                  }}
                />
              </div>
            )}
            <span className="muted">{t("rawReviewCompare.decided", { count: decided })}</span>
            <span className="muted rr-footer-hint">{t("rawReviewCompare.hint")}</span>
            {/* Scelta e stelle date da tastiera si leggono anche senza guardare i pannelli. */}
            <span className="visually-hidden" role="status" data-testid="rr-announce">
              {announcement}
            </span>
          </nav>
        </div>
        <InfoSidebar
          candidates={candidates}
          panels={panels}
          guidePreset={opened.guidePreset}
          shot={shot}
          metadata={metadata}
          onToggle={(preset) => void toggleVisible(preset)}
          onMove={(preset, to) => void movePreset(preset, to)}
          onShowAll={() => void showAllPresets()}
        />
      </div>
    </div>
  );
}

/**
 * Testo per gli screen reader quando cambiano la variante migliore o le stelle dello scatto
 * mostrato. Passare a un altro scatto non annuncia nulla qui: lo annuncia già la posizione.
 */
function useDecisionAnnouncement(
  shotKey: string | undefined,
  decision: ShotDecision | undefined,
): string {
  const { t } = useTranslation();
  const rating = decision?.rating ?? 0;
  const winner = decision?.winnerPreset ?? null;
  const [state, setState] = useState({ shotKey, rating, winner, text: "" });
  if (state.shotKey !== shotKey) {
    setState({ shotKey, rating, winner, text: "" });
  } else if (state.rating !== rating || state.winner !== winner) {
    const parts = [
      rating > 0 ? t("rawReviewShots.stars", { count: rating }) : t("rawReviewCompare.noStars"),
    ];
    if (winner) parts.push(t("rawReviewShots.best", { name: presetLabel(t, winner) }));
    setState({ shotKey, rating, winner, text: parts.join(", ") });
  }
  return state.text;
}
