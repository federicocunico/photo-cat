import { useTranslation } from "react-i18next";
import { ModulePage } from "../../components/shell/ModulePage";
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
import { useRatingHotkeys } from "./useRatingHotkeys";
import { FirstSteps } from "./FirstSteps";
import { useSystemCatalogCommands } from "./useSystemCatalogCommands";
import { DragGhost } from "../../components/dnd/photoDrag";
import "./library.css";

export function LibraryPage() {
  const { t } = useTranslation();
  // Esiti di esclusioni e reinserimenti, anche mentre la barra delle azioni è visibile.
  useCatalogFeedback();
  useSystemCatalogCommands();
  const selected = useLibrary((s) => selCount(s.selection));
  useTagHotkeys(() => {
    const s = useLibrary.getState().selection;
    return { target: selTarget(s), count: selCount(s) };
  });
  useRatingHotkeys(() => {
    const s = useLibrary.getState().selection;
    return { target: selTarget(s), count: selCount(s) };
  });
  // Titolo del modulo come negli altri; il contesto (catalogo, filtri, numero di foto) resta nella
  // barra della vista sopra la griglia.
  return (
    <ModulePage title={t("modules.library")} scroll={false}>
      <DragGhost />
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
    </ModulePage>
  );
}
