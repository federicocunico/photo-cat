import { create } from "zustand";

/**
 * Conferme fuori dal flusso di un componente (al posto di `window.confirm`, vietato da
 * ui-foundations, "Conferme"): `askConfirm` apre il ConfirmDialog dell'unico
 * `ConfirmHost` e risolve con la scelta dell'utente.
 */
export type ConfirmRequest = {
  title: string;
  message?: string;
  /** Verbo esplicito, mai "OK". */
  confirmLabel: string;
  count?: number;
  files?: "untouched" | "affected";
  danger?: boolean;
};

type State = { request: (ConfirmRequest & { resolve: (ok: boolean) => void }) | null };

export const useConfirm = create<State>(() => ({ request: null }));

export function askConfirm(request: ConfirmRequest): Promise<boolean> {
  // Una nuova richiesta chiude la precedente come annullata.
  useConfirm.getState().request?.resolve(false);
  return new Promise((resolve) => useConfirm.setState({ request: { ...request, resolve } }));
}

export function answerConfirm(ok: boolean) {
  const request = useConfirm.getState().request;
  if (!request) return;
  useConfirm.setState({ request: null });
  request.resolve(ok);
}
