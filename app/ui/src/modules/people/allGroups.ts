import { commands, type GroupRow, type GroupSort, type PeopleSection } from "../../lib/ipc";

const PAGE = 200;
/** Limite di sicurezza: 200 pagine da 200 gruppi coprono 40.000 gruppi. */
const MAX_PAGES = 200;

/**
 * Tutti i gruppi di una sezione, pagina per pagina: le destinazioni di «Sposta in…» e
 * dell'unione non devono fermarsi ai primi gruppi dell'elenco.
 */
export async function listAllGroups(section: PeopleSection, sort: GroupSort): Promise<GroupRow[]> {
  const out: GroupRow[] = [];
  const seen = new Set<number>();
  let after: number | null = null;
  for (let i = 0; i < MAX_PAGES; i++) {
    const page = await commands.listGroups(section, null, sort, after, PAGE);
    for (const g of page.items) {
      if (seen.has(g.id)) continue;
      seen.add(g.id);
      out.push(g);
    }
    if (page.next == null || page.next === after) break;
    after = page.next;
  }
  return out;
}
