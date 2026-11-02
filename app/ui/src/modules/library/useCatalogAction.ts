import { create } from "zustand";
import { commands } from "../../lib/ipc";

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
  const act = async (ids: number[], catalog: number, name: string, restore = false) => {
    if (useCatalogOperation.getState().busy || !ids.length) return;
    useCatalogOperation.setState({
      busy: true,
      job: null,
      catalog: name,
      count: ids.length,
      restore,
      message: "",
    });
    try {
      const job = await commands.startBulk(
        ids,
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
