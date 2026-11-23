import { useState } from "react";
import { faceUrl, thumbUrl } from "../library/thumbUrl";

/**
 * Volto di copertina di un gruppo o di una persona (elenco, selettore, scheda, coda, storico).
 * Basta l'id del volto; la miniatura della foto, se c'è, fa da segnaposto.
 */
export function Cover({
  cover,
  size = "sm",
}: {
  cover: { id: number; thumb?: string | null } | null;
  size?: "sm" | "md";
}) {
  const [failed, setFailed] = useState(false);
  const src = cover ? faceUrl(cover.id) : null;
  const placeholder = cover?.thumb ? thumbUrl(cover.thumb) : null;
  return (
    <span
      aria-hidden
      className={`people-cover people-cover--${size}`}
      style={{
        backgroundImage: placeholder && !src ? `url("${placeholder}")` : undefined,
      }}
    >
      {src && !failed && (
        <img
          src={src}
          alt=""
          loading="lazy"
          decoding="async"
          onError={() => setFailed(true)}
          className="people-cover-img"
        />
      )}
    </span>
  );
}
