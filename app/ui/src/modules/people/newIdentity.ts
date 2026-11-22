import { commands } from "../../lib/ipc";

/**
 * "Nuova persona…": crea prima l'identità (come "Chi è?", così la persona compare anche in
 * "Persone"), poi esegue `assign` verso di essa. Se l'assegnazione fallisce l'identità appena
 * creata viene eliminata, per non lasciare un nome vuoto nell'elenco.
 */
export async function withNewIdentity<T>(
  name: string,
  assign: (uid: string) => Promise<T>,
): Promise<T> {
  const uid = await commands.saveIdentity(null, {
    name,
    shortName: null,
    group: null,
    threshold: null,
    aliases: [],
  });
  try {
    return await assign(uid);
  } catch (e) {
    await commands.deleteIdentity(uid).catch(() => undefined);
    throw e;
  }
}
