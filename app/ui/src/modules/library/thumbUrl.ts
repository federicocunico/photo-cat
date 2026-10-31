import { convertFileSrc } from "@tauri-apps/api/core";
import { isTauri } from "../../lib/ipc";

/** URL della miniatura servita dal protocollo `thumb://` del backend. */
export function thumbUrl(key: string | null | undefined): string | null {
  if (!key || !isTauri()) return null;
  return convertFileSrc(key, "thumb");
}
