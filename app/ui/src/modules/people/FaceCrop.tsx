import { useState } from "react";
import type { FaceView } from "../../lib/ipc";
import { faceUrl, thumbUrl } from "../library/thumbUrl";

/** Ritaglio volto: miniatura immediata, poi crop ad alta risoluzione. */
export function FaceCrop({ face, size = 112 }: { face: FaceView; size?: number }) {
  const [x, y, w, h] = face.bbox.map((v) => v ?? 0);
  const [failed, setFailed] = useState(false);
  const src = faceUrl(face.id);
  return (
    <span
      style={{
        display: "block",
        position: "relative",
        overflow: "hidden",
        width: size,
        height: size,
        backgroundColor: "var(--pc-panel)",
        backgroundImage: face.thumb ? `url("${thumbUrl(face.thumb)}")` : undefined,
        backgroundSize: `${100 / Math.max(w, 0.01)}% ${100 / Math.max(h, 0.01)}%`,
        backgroundPosition: `${(100 * x) / Math.max(1 - w, 0.01)}% ${(100 * y) / Math.max(1 - h, 0.01)}%`,
        borderRadius: 6,
      }}
    >
      {src && !failed && (
        <img
          src={src}
          alt=""
          loading="lazy"
          decoding="async"
          onError={() => setFailed(true)}
          style={{
            position: "absolute",
            inset: 0,
            width: "100%",
            height: "100%",
            objectFit: "cover",
          }}
        />
      )}
    </span>
  );
}
