import type { PeopleSection } from "../../lib/ipc";

/** Id della scheda di una sezione di Persone (per `aria-labelledby` del pannello). */
export const peopleTabId = (s: PeopleSection) => `people-tab-${s}`;
