// Visualizzatore delle foto condiviso da Libreria e Sequenza (desktop-shell, "Visualizzatore
// delle foto" e "Comandi del visualizzatore"): frecce per scorrere nell'ordine della vista,
// 1–9 e 0 per i tag del set attivo, I per le informazioni, Esc o Spazio per tornare.
import * as RDialog from "@radix-ui/react-dialog";
import { useState, type KeyboardEvent, type ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { Icon } from "../icons";
import "./viewer.css";

export type ViewerPhoto = {
  id: number;
  name: string;
  /** URL dell'immagine più grande disponibile (miniatura, design: Open Questions 1). */
  src: string | null;
  /** Riga di informazioni in parole (data, luogo, dispositivo…). */
  details?: ReactNode;
};

export function Viewer({
  photo,
  index,
  total,
  onIndex,
  onClose,
  onKey,
}: {
  /** Foto mostrata; `null` se non ancora caricata. */
  photo: ViewerPhoto | null;
  index: number;
  total: number;
  /** Nuova posizione nella vista (frecce). */
  onIndex: (index: number) => void;
  onClose: () => void;
  /** Altri tasti (es. 1–9 e 0 per i tag): restituisce `true` se il tasto è stato usato. */
  onKey?: (key: string) => boolean;
}) {
  const { t } = useTranslation();
  const [info, setInfo] = useState(false);
  const go = (delta: number) => {
    const next = Math.max(0, Math.min(total - 1, index + delta));
    if (next !== index) onIndex(next);
  };
  const onKeyDown = (e: KeyboardEvent) => {
    if (e.ctrlKey || e.metaKey || e.altKey) return;
    const handled = (() => {
      switch (e.key) {
        case "ArrowRight":
        case "ArrowDown":
          go(1);
          return true;
        case "ArrowLeft":
        case "ArrowUp":
          go(-1);
          return true;
        case "Home":
          onIndex(0);
          return true;
        case "End":
          onIndex(total - 1);
          return true;
        case " ":
          onClose();
          return true;
        case "i":
        case "I":
          setInfo((v) => !v);
          return true;
        default:
          return onKey?.(e.key) ?? false;
      }
    })();
    if (handled) {
      // I gestori globali (tasti dei tag della pagina, Esc della selezione) non lo vedono.
      e.preventDefault();
      e.stopPropagation();
    }
  };
  const title = photo?.name ?? t("viewer.loading");
  return (
    <RDialog.Root open onOpenChange={(open) => !open && onClose()}>
      <RDialog.Portal>
        <RDialog.Overlay className="viewer-overlay" />
        <RDialog.Content
          className="viewer"
          aria-describedby={undefined}
          onKeyDown={onKeyDown}
          data-testid="viewer"
        >
          <header className="viewer-bar">
            <RDialog.Title className="viewer-title">{title}</RDialog.Title>
            <span className="viewer-count">{t("viewer.position", { n: index + 1, total })}</span>
            <button
              type="button"
              className="viewer-button"
              aria-pressed={info}
              aria-keyshortcuts="I"
              onClick={() => setInfo((v) => !v)}
            >
              <Icon.Info size={16} aria-hidden="true" />
              {t("viewer.info")}
            </button>
            <RDialog.Close className="viewer-button" aria-keyshortcuts="Escape">
              <Icon.Close size={16} aria-hidden="true" />
              {t("viewer.close")}
            </RDialog.Close>
          </header>
          <div className="viewer-stage">
            <button
              type="button"
              className="viewer-nav"
              aria-label={t("viewer.previous")}
              disabled={index <= 0}
              onClick={() => go(-1)}
            >
              <Icon.ChevronLeft size={28} aria-hidden="true" />
            </button>
            {photo?.src ? (
              <img className="viewer-image" src={photo.src} alt={photo.name} />
            ) : (
              <span className="viewer-missing">{photo?.name ?? t("viewer.loading")}</span>
            )}
            <button
              type="button"
              className="viewer-nav"
              aria-label={t("viewer.next")}
              disabled={index >= total - 1}
              onClick={() => go(1)}
            >
              <Icon.ChevronRight size={28} aria-hidden="true" />
            </button>
          </div>
          {info && photo?.details && <footer className="viewer-info">{photo.details}</footer>}
        </RDialog.Content>
      </RDialog.Portal>
    </RDialog.Root>
  );
}
