// Barra laterale dei pannelli e delle informazioni dello scatto (porting di FujiView
// `InfoSidebar.tsx`): elenco dei preset (mostra/nascondi e ordine) e sezioni dei metadati
// Scatto, Fotocamera, Luogo e File della variante guida.
import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { Icon } from "../../components/icons";
import { Button, Disclosure, IconButton, Kbd } from "../../components/ui";
import type { ImageMetadata, ReviewShotView } from "../../lib/ipc";
import { metaLabel, metaSectionTitle, metaValue, recipeSummary } from "./metaText";
import { presetLabel } from "./presetLabel";
import { MAX_PANELS, panelLock } from "./project";

type Props = {
  /** Tutti i pannelli che si possono mostrare, nell'ordine predefinito. */
  candidates: readonly string[];
  panels: readonly string[];
  guidePreset: string | null;
  shot: ReviewShotView | undefined;
  metadata: Record<string, ImageMetadata>;
  onToggle: (preset: string) => void;
  onMove: (preset: string, to: number) => void;
  onShowAll: () => void;
};

export function InfoSidebar({
  candidates,
  panels,
  guidePreset,
  shot,
  metadata,
  onToggle,
  onMove,
  onShowAll,
}: Props) {
  const { t } = useTranslation();
  // I dati di scatto sono uguali in ogni variante: si leggono dalla guida, altrimenti dalla prima.
  const primaryPreset = shot?.variants.some((v) => v.preset === guidePreset)
    ? guidePreset
    : shot?.variants[0]?.preset;
  const primary = primaryPreset ? metadata[primaryPreset] : undefined;
  const coordinates =
    primary?.gps?.latitude != null && primary.gps.longitude != null
      ? `${primary.gps.latitude.toFixed(6)}, ${primary.gps.longitude.toFixed(6)}`
      : null;
  const copied = useCopied();

  return (
    <aside className="rr-info" aria-label={t("rawReviewInfo.region")}>
      <Disclosure
        title={
          <>
            {t("rawReviewInfo.presets")}
            <small className="muted rr-info-count">
              {t("rawReviewInfo.visibleCount", {
                shown: panels.length,
                total: candidates.length,
              })}
            </small>
          </>
        }
      >
        <ul className="rr-presets">
          {candidates.map((preset) => {
            const position = panels.indexOf(preset);
            const shown = position >= 0;
            const lock = panelLock(panels, preset);
            const recipe = metadata[preset]?.recipe;
            const missing = !!shot && !shot.variants.some((v) => v.preset === preset);
            const name = presetLabel(t, preset);
            const reason =
              lock === "last"
                ? t("rawReviewCompare.lastPanel")
                : lock === "max"
                  ? t("rawReviewInfo.maxPanels", { max: MAX_PANELS })
                  : undefined;
            return (
              <li key={preset} className={`rr-preset${shown ? " is-shown" : ""}`}>
                <label className="rr-preset-toggle" title={reason}>
                  <input
                    type="checkbox"
                    checked={shown}
                    disabled={lock !== null}
                    onChange={() => onToggle(preset)}
                  />
                  <span className="rr-preset-name">
                    <span>
                      {name}
                      {preset === guidePreset && (
                        <small className="muted"> · {t("rawReviewInfo.guide")}</small>
                      )}
                    </span>
                    {missing ? (
                      <small className="rr-preset-missing">{t("rawReviewInfo.missing")}</small>
                    ) : (
                      recipe && <small className="muted">{recipeSummary(t, recipe)}</small>
                    )}
                    {reason && <span className="visually-hidden">{reason}</span>}
                  </span>
                </label>
                {shown && (
                  <span className="rr-preset-order">
                    <Kbd keys={[String(position + 1)]} />
                    <IconButton
                      size="sm"
                      label={t("rawReviewInfo.moveUp", { name })}
                      icon={<Icon.ArrowUp size={14} />}
                      disabled={position === 0}
                      onClick={() => onMove(preset, position - 1)}
                    />
                    <IconButton
                      size="sm"
                      label={t("rawReviewInfo.moveDown", { name })}
                      icon={<Icon.ArrowDown size={14} />}
                      disabled={position === panels.length - 1}
                      onClick={() => onMove(preset, position + 1)}
                    />
                  </span>
                )}
              </li>
            );
          })}
        </ul>
        {panels.length < Math.min(candidates.length, MAX_PANELS) && (
          <Button size="sm" onClick={onShowAll}>
            {t("rawReviewInfo.showAll")}
          </Button>
        )}
      </Disclosure>

      {!shot && <p className="muted rr-info-empty">{t("rawReviewInfo.noShot")}</p>}
      {shot && !primary && <p className="muted rr-info-empty">{t("rawReviewInfo.loading")}</p>}
      {primary?.sections.map((section) => (
        <Disclosure key={section.title} title={metaSectionTitle(t, section)}>
          <dl className="rr-info-rows">
            {section.rows.map((row) => {
              const value = metaValue(t, row);
              return (
                <div key={row.label}>
                  <dt>{metaLabel(t, row)}</dt>
                  <dd title={value}>{value}</dd>
                </div>
              );
            })}
          </dl>
          {section.title === "place" && coordinates && (
            <Button
              size="sm"
              icon={<Icon.Copy size={14} />}
              onClick={() => copied.copy(coordinates)}
            >
              {copied.done ? t("rawReviewInfo.copied") : t("rawReviewInfo.copyCoordinates")}
            </Button>
          )}
        </Disclosure>
      ))}
    </aside>
  );
}

/** Copia negli appunti e mostra "Copiate" per un istante; senza appunti non fa nulla. */
function useCopied() {
  const [done, setDone] = useState(false);
  const timer = useRef<number | undefined>(undefined);
  useEffect(() => () => window.clearTimeout(timer.current), []);
  const copy = async (text: string) => {
    try {
      await navigator.clipboard.writeText(text);
      setDone(true);
      window.clearTimeout(timer.current);
      timer.current = window.setTimeout(() => setDone(false), 1500);
    } catch {
      // Appunti non disponibili: niente da segnalare.
    }
  };
  return { done, copy };
}
