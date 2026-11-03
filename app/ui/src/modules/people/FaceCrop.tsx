import { useState } from "react";
import type { FaceView } from "../../lib/ipc";
import { faceUrl, thumbUrl } from "../library/thumbUrl";
import "./people.css";

/** Ritaglio volto: miniatura immediata, poi crop ad alta risoluzione. */
export function FaceCrop({ face, size = 112 }: { face: FaceView; size?: number }) {
  const [x, y, w, h] = face.bbox.map((v) => v ?? 0);
  const [failed, setFailed] = useState(false);
  const src = faceUrl(face.id);
  return (
    <span
      className="people-facecrop"
      style={{
        width: size,
        height: size,
        backgroundImage: face.thumb ? `url("${thumbUrl(face.thumb)}")` : undefined,
        backgroundSize: `${100 / Math.max(w, 0.01)}% ${100 / Math.max(h, 0.01)}%`,
        backgroundPosition: `${(100 * x) / Math.max(1 - w, 0.01)}% ${(100 * y) / Math.max(1 - h, 0.01)}%`,
      }}
    >
      {src && !failed && (
        <img
          src={src}
          alt=""
          loading="lazy"
          decoding="async"
          onError={() => setFailed(true)}
          className="people-facecrop-img"
        />
      )}
    </span>
  );
}
