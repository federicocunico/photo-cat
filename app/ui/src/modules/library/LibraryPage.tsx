import { Panels } from "../../components/shell/Panels";
import { selCount, selTarget, useLibrary } from "../../store/library";
import { LibraryGrid } from "./Grid";
import { Inspector } from "./Inspector";
import { LibrarySidebar } from "./Sidebar";
import { BulkBar } from "./BulkBar";
import { FacetPanels } from "./FacetPanel";
import { LibraryHeader } from "./LibraryHeader";
import { useTagHotkeys } from "../tags/useTagHotkeys";

export function LibraryPage() {
  useTagHotkeys(() => {
    const s = useLibrary.getState().selection;
    return { target: selTarget(s), count: selCount(s) };
  });
  return (
    <Panels
      left={<LibrarySidebar />}
      center={
        <>
          <LibraryHeader />
          <BulkBar />
          <LibraryGrid />
          <FacetPanels />
        </>
      }
      right={<Inspector />}
    />
  );
}
