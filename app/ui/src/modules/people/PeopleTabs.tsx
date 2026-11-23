import { useQuery } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { commands, isTauri, type PeopleSection } from "../../lib/ipc";
import { fmtInt } from "../../lib/format";
import { peopleTabId } from "./peopleTabId";

const SECTIONS: PeopleSection[] = ["review", "people", "ignored"];

/**
 * Schede "Da rivedere / Persone / Ignorati" con il conteggio, su una riga nell'intestazione della
 * pagina (desktop-shell, "Revisione delle persone"; ui-usability-audit D1).
 */
export function PeopleTabs({
  section,
  onChange,
}: {
  section: PeopleSection;
  onChange: (s: PeopleSection) => void;
}) {
  const { t } = useTranslation();
  const sections = useQuery({
    queryKey: ["people-sections"],
    queryFn: commands.peopleSections,
    enabled: isTauri(),
  });
  // Nella sezione Persone il conteggio è quello delle persone mostrate (anche senza foto qui).
  const identities = useQuery({
    queryKey: ["identities", "", null],
    queryFn: () => commands.listIdentities(null, null),
    enabled: isTauri() && section === "people",
  });
  const counts = sections.data;
  const count = (s: PeopleSection) =>
    s === "people"
      ? (identities.data?.identities.length ?? counts?.people ?? 0)
      : ((s === "review" ? counts?.review : counts?.ignored) ?? 0);
  const label = (s: PeopleSection) =>
    t(
      s === "review"
        ? "people.sectionReview"
        : s === "people"
          ? "people.sectionPeople"
          : "people.sectionIgnored",
    );
  return (
    <div role="tablist" aria-label={t("people.title")} className="people-tabs">
      {SECTIONS.map((s) => (
        <button
          key={s}
          id={peopleTabId(s)}
          type="button"
          role="tab"
          aria-selected={section === s}
          aria-controls="people-workspace"
          tabIndex={section === s ? 0 : -1}
          className="btn people-tab"
          onClick={() => onChange(s)}
          onKeyDown={(e) => {
            if (e.key !== "ArrowRight" && e.key !== "ArrowLeft") return;
            e.preventDefault();
            const i = SECTIONS.indexOf(s) + (e.key === "ArrowRight" ? 1 : -1);
            const next = SECTIONS[(i + SECTIONS.length) % SECTIONS.length]!;
            onChange(next);
            document.getElementById(peopleTabId(next))?.focus();
          }}
        >
          <span className="people-tab-label">{label(s)}</span>
          <span className="muted">{fmtInt(count(s))}</span>
        </button>
      ))}
    </div>
  );
}
