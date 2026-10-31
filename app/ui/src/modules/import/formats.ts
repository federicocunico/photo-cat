/** Gruppi di formati mostrati all'utente → estensioni. */
export const FORMAT_GROUPS: Array<{ id: string; label: string; ext: string[]; def: boolean }> = [
  { id: "jpg", label: "JPG", ext: ["jpg", "jpeg"], def: true },
  { id: "png", label: "PNG", ext: ["png"], def: true },
  { id: "tiff", label: "TIFF", ext: ["tif", "tiff"], def: true },
  { id: "webp", label: "WebP", ext: ["webp"], def: true },
  { id: "bmp", label: "BMP · GIF", ext: ["bmp", "gif"], def: true },
  { id: "heic", label: "HEIC · HEIF · AVIF", ext: ["heic", "heif", "avif"], def: true },
  {
    id: "raw",
    label: "RAW (RAF, CR2, CR3, NEF, ARW, DNG…)",
    ext: ["dng", "cr2", "cr3", "nef", "arw", "orf", "rw2", "raf", "pef", "srw"],
    def: false,
  },
];

export const splitPatterns = (s: string) =>
  s
    .split(/[;\n]/)
    .map((p) => p.trim())
    .filter(Boolean);
