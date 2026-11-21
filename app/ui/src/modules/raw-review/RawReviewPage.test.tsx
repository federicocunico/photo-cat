import { afterEach, describe, expect, it } from "vitest";
import { cleanup, render, screen } from "@testing-library/react";
import "../../i18n";
import { RawReviewPage } from "./RawReviewPage";

afterEach(cleanup);

describe("Varianti RAW", () => {
  it("mostra il titolo e lo stato vuoto", () => {
    render(<RawReviewPage />);
    expect(
      screen.getByRole("heading", { level: 1, name: "Varianti RAW (Fujifilm X-T5)" }),
    ).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "Nessuna cartella aperta" })).toBeInTheDocument();
  });
});
