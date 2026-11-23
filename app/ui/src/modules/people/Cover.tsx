import { useState } from "react";
import type { GroupRow } from "../../lib/ipc";
import { faceUrl, thumbUrl } from "../library/thumbUrl";

/** Volto di copertina di un gruppo o di una persona (elenco, selettore, scheda). */
export function Cover({ cover, size = "sm" }: { cover: GroupRow["cover"]; size?: "sm" | "md" }) {
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
