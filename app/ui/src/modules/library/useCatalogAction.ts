import { create } from "zustand";
import { commands, type Target } from "../../lib/ipc";

export const useCatalogOperation = create<{
  busy: boolean;
  job: number | null;
  catalog: string;
  count: number;
  restore: boolean;
  message: string;
  undoable: boolean;
}>(() => ({
  busy: false,
  job: null,
  catalog: "",
  count: 0,
  restore: false,
  message: "",
  undoable: false,
}));

export function useCatalogAction() {
  const busy = useCatalogOperation((s) => s.busy);
  /** `items`: id espliciti, oppure il bersaglio di una selezione "tutti tranne…" con il conteggio. */
  const act = async (
    items: number[] | { target: Target; count: number },
    catalog: number,
    name: string,
    restore = false,
  ) => {
    const { target, count } = Array.isArray(items)
      ? { target: { kind: "ids", ids: items } as Target, count: items.length }
      : items;
    if (useCatalogOperation.getState().busy || !count) return;
    useCatalogOperation.setState({
      busy: true,
      job: null,
      catalog: name,
      count,
      restore,
      message: "",
    });
    try {
      const job = await commands.startBulk(
        target,
        {
          action: restore ? "restore_to_catalog" : "exclude_from_catalog",
          catalog,
        },
        false,
      );
      useCatalogOperation.setState({ job });
    } catch (error) {
      useCatalogOperation.setState({ busy: false, message: String(error), undoable: false });
    }
  };
  return { busy, act };
}
