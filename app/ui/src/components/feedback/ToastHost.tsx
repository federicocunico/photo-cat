import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { useToast, type Notice } from "../../store/toasts";
import { Icon } from "../icons";
import { BusyButton } from "./BusyButton";

/**
 * Un avviso: si chiude da solo dopo `duration`, ma il tempo si ferma finché il puntatore o
 * il focus sono sopra (così c'è tempo per "Annulla operazione").
 */
function ToastItem({ notice, onClose }: { notice: Notice; onClose: () => void }) {
  const { t } = useTranslation();
  const close = useRef(onClose);
  close.current = onClose;
  const [paused, setPaused] = useState(false);
  const left = useRef(notice.duration);
  useEffect(() => {
    if (left.current == null || paused) return;
    const start = Date.now();
    const timer = setTimeout(() => close.current(), left.current);
    return () => {
      clearTimeout(timer);
      if (left.current != null) left.current = Math.max(0, left.current - (Date.now() - start));
    };
  }, [paused]);
  const error = notice.tone === "error";
  return (
    <div
      className={`feedback-toast feedback-toast-${notice.tone}`}
      role={error ? "alert" : "status"}
      onPointerEnter={() => setPaused(true)}
      onPointerLeave={() => setPaused(false)}
      onFocus={() => setPaused(true)}
      onBlur={(e) => {
        if (!e.currentTarget.contains(e.relatedTarget as Node | null)) setPaused(false);
      }}
    >
      <span className="feedback-toast-icon">
        {error ? (
          <Icon.Alert size={16} />
        ) : notice.tone === "ok" ? (
          <Icon.Check size={16} />
        ) : (
          <Icon.Info size={16} />
        )}
      </span>
      <span className="feedback-toast-text">{notice.message}</span>
      {notice.action && (
        <BusyButton
          className="btn"
          onClick={async () => {
            await notice.action!.run();
            onClose();
          }}
        >
          {notice.action.label}
        </BusyButton>
      )}
      <button
        type="button"
        className="btn btn-ghost icon-btn"
        aria-label={t("common.dismiss")}
        onClick={onClose}
      >
        <Icon.Close size={14} />
      </button>
    </div>
  );
}

/**
 * Unico punto in cui compaiono gli avvisi. Se il focus finisce su un elemento coperto
 * dagli avvisi (in basso a destra), la pila si sposta in alto: il focus non resta mai
 * nascosto (WCAG 2.4.11).
 */
export function ToastHost() {
  const { notices, dismiss } = useToast();
  const host = useRef<HTMLDivElement>(null);
  const [top, setTop] = useState(false);
  useEffect(() => {
    const onFocus = (e: FocusEvent) => {
      const el = e.target as HTMLElement | null;
      const box = host.current;
      if (!el || !box || box.contains(el) || typeof el.getBoundingClientRect !== "function") return;
      const a = el.getBoundingClientRect();
      const b = box.getBoundingClientRect();
      const empty = b.width === 0 || b.height === 0;
      const overlaps =
        !empty && a.left < b.right && a.right > b.left && a.top < b.bottom && a.bottom > b.top;
      // In alto solo finché serve: un focus nella metà superiore la riporta in basso.
      if (overlaps) setTop(true);
      else if (a.bottom < window.innerHeight / 2) setTop(false);
    };
    document.addEventListener("focusin", onFocus);
    return () => document.removeEventListener("focusin", onFocus);
  }, []);
  return (
    <div ref={host} className="feedback-toasts" data-position={top ? "top" : "bottom"}>
      {notices.map((n) => (
        <ToastItem key={n.id} notice={n} onClose={() => dismiss(n.id)} />
      ))}
    </div>
  );
}
