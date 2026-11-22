import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, expect, it } from "vitest";
import i18n from "../../i18n";
import { guide } from "../../i18n/guide";
import { useGuide } from "../../store/guide";
import { GuideDialog } from "./GuideDialog";

beforeEach(() => useGuide.getState().openGuide(null));
afterEach(async () => {
  cleanup();
  await act(() => i18n.changeLanguage("it"));
});

it("la sezione Varianti RAW ha titolo e testi in italiano e in inglese", () => {
  for (const lang of ["it", "en"] as const) {
    const section = guide[lang].guide.sections.rawReview;
    expect(section.title).toContain("RAW");
    for (const word of ["Ctrl+9", "Maiusc", "Shift", "Windows", "Preset Fujifilm", "RAF"]) {
      // Ogni lingua cita i punti chiave: scorciatoia, sviluppo solo su Windows, export e RAF.
      if (lang === "it" && word === "Shift") continue;
      if (lang === "en" && word === "Maiusc") continue;
      expect(section.body).toContain(word);
    }
  }
  expect(guide.it.guide.sections.rawReview.body).not.toEqual(
    guide.en.guide.sections.rawReview.body,
  );
});

it("Aiuto › Guida mostra la sezione Varianti RAW, a paragrafi, nella lingua dell'app", async () => {
  useGuide.getState().openGuide("rawReview");
  render(<GuideDialog />);
  const heading = await screen.findByRole("heading", { name: "Varianti RAW (Fujifilm X-T5)" });
  const section = heading.closest("section")!;
  expect(section).toHaveAttribute("data-current", "true");
  expect(section.querySelectorAll("p").length).toBeGreaterThan(4);
  expect(section.textContent).toContain("USB RAW CONV./BACKUP RESTORE");
  await act(() => i18n.changeLanguage("en"));
  expect(screen.getByRole("heading", { name: "RAW variants (Fujifilm X-T5)" })).toBeInTheDocument();
  fireEvent.keyDown(document.activeElement ?? document.body, { key: "Escape" });
});
