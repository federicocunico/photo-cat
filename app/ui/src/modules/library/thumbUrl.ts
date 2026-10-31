import { convertFileSrc } from "@tauri-apps/api/core";
import { isTauri } from "../../lib/ipc";

/** URL della miniatura servita dal protocollo `thumb://` del backend. */
export function thumbUrl(key: string | null | undefined): string | null {
  if (!key || !isTauri()) return null;
  return convertFileSrc(key, "thumb");
}

/** URL del ritaglio del volto servito dal protocollo `face://` del backend. */
export function faceUrl(id: number): string | null {
  return isTauri() ? convertFileSrc(String(id), "face") : null;
}
