// Dialoghi di sistema del modulo Varianti RAW; sostituibili nei test con vi.mock del plugin.

/** Sceglie un progetto FujiView (`.fujiview`); `null` se l'utente annulla. */
export async function chooseProject(): Promise<string | null> {
  const { open } = await import("@tauri-apps/plugin-dialog");
  const p = await open({
    multiple: false,
    directory: false,
    filters: [{ name: "FujiView", extensions: ["fujiview"] }],
  });
  return typeof p === "string" ? p : null;
}
