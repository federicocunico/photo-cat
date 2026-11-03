import { Panels } from "../../components/shell/Panels";
import { selCount, selTarget, useLibrary } from "../../store/library";
import { LibraryGrid } from "./Grid";
import { Inspector } from "./Inspector";
import { LibrarySidebar } from "./Sidebar";
import { SelectionBar } from "./SelectionBar";
import { useCatalogFeedback } from "./catalogActions";
import { FacetPanels } from "./FacetPanel";
import { LibraryHeader } from "./LibraryHeader";
import { useTagHotkeys } from "../tags/useTagHotkeys";
import { FirstSteps } from "./FirstSteps";

export function LibraryPage() {
  // Esiti di esclusioni e reinserimenti, anche mentre la barra delle azioni è visibile.
  useCatalogFeedback();
  const selected = useLibrary((s) => selCount(s.selection));
  useTagHotkeys(() => {
    const s = useLibrary.getState().selection;
    return { target: selTarget(s), count: selCount(s) };
  });
  return (
    <Panels
      left={<LibrarySidebar />}
      center={
        <>
          {selected > 0 ? <SelectionBar /> : <LibraryHeader />}
          <FirstSteps />
          <LibraryGrid />
          <FacetPanels />
        </>
      }
      right={<Inspector />}
    />
  );
}
