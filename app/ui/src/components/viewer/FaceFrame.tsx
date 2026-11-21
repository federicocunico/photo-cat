// Foto del visore con il riquadro di un volto evidenziato (bbox normalizzata 0..1).
import { useEffect, useRef, useState } from "react";

type Bbox = [number | null, number | null, number | null, number | null];

export function FaceFrame({ src, alt, bbox }: { src: string; alt: string; bbox: Bbox }) {
  const wrap = useRef<HTMLDivElement>(null);
  const [size, setSize] = useState({ w: 0, h: 0 });
  const [nat, setNat] = useState({ w: 0, h: 0 });
  useEffect(() => {
    const el = wrap.current;
    if (!el) return;
    const read = () => setSize({ w: el.clientWidth, h: el.clientHeight });
    read();
    if (typeof ResizeObserver === "undefined") return;
    const ro = new ResizeObserver(read);
    ro.observe(el);
    return () => ro.disconnect();
  }, []);
  // Con object-fit: contain la foto occupa solo una parte del riquadro: se ne calcola l'area.
  let box: { left: number; top: number; width: number; height: number } | null = null;
  if (size.w > 0 && size.h > 0 && nat.w > 0 && nat.h > 0) {
    const k = Math.min(size.w / nat.w, size.h / nat.h);
    const iw = nat.w * k;
    const ih = nat.h * k;
    const [x, y, w, h] = bbox.map((v) => v ?? 0);
    box = {
      left: (size.w - iw) / 2 + x * iw,
      top: (size.h - ih) / 2 + y * ih,
      width: w * iw,
      height: h * ih,
    };
  }
  return (
    <div ref={wrap} className="viewer-frame">
      <img
        className="viewer-frame-image"
        src={src}
        alt={alt}
        onLoad={(e) =>
          setNat({ w: e.currentTarget.naturalWidth, h: e.currentTarget.naturalHeight })
        }
      />
      {box && <div className="viewer-face-box" data-testid="viewer-face-box" style={box} />}
    </div>
  );
}
