// Modulo AI (ai-workspace D8): sotto-navigazione a sezioni come in Impostazioni. La
// sezione attiva sta in `layout` (si ricorda tra le aperture); `openModule({ module: "ai",
// section })` apre una sezione precisa.
import { useTranslation } from "react-i18next";
import { PageHeader } from "../../components/ui";
import { useLayout, type AiSection } from "../../store/layout";
import { AI_SECTIONS } from "./aiSections";
import { CleanupSection } from "./CleanupSection";
import { OverviewSection } from "./OverviewSection";
import { ReprocessSection } from "./ReprocessSection";
import { ResultsSection } from "./ResultsSection";
import { MapSection } from "./map/MapSection";
import { SimilarSection } from "./SimilarSection";
import "./ai.css";

function SectionContent({ section }: { section: AiSection }) {
  switch (section) {
    case "reprocess":
      return <ReprocessSection />;
    case "results":
      return <ResultsSection />;
    case "cleanup":
      return <CleanupSection />;
    case "map":
      return <MapSection />;
    case "similar":
      return <SimilarSection />;
    default:
      return <OverviewSection />;
  }
}

export function AiPage() {
  const { t } = useTranslation();
  const stored = useLayout((s) => s.aiSection);
  const setSection = useLayout((s) => s.setAiSection);
  // Una sezione non mostrata ricade su Panoramica.
  const section = AI_SECTIONS.includes(stored) ? stored : "overview";
  return (
    <div className="ai-page">
      <PageHeader title={t("modules.ai")} />
      <div className="ai-body">
        <aside className="ai-aside">
          <nav aria-label={t("ai.sections.label")}>
            <ul className="ai-nav">
              {AI_SECTIONS.map((s) => (
                <li key={s}>
                  <button
                    type="button"
                    className="ai-nav-item"
                    aria-current={s === section ? "page" : undefined}
                    onClick={() => setSection(s)}
                  >
                    {t(`ai.sections.${s}`)}
                  </button>
                </li>
              ))}
            </ul>
          </nav>
        </aside>
        <div className="ai-main">
          <SectionContent section={section} />
        </div>
      </div>
    </div>
  );
}
