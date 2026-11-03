import { afterEach, describe, expect, it } from "vitest";
import { cleanup, render } from "@testing-library/react";
import { auditA11y, expectAccessible } from "./a11y";

afterEach(cleanup);

describe("controllo di accessibilità del DOM", () => {
  it("legge il minimo dalle classi dell'app", () => {
    const { container } = render(
      <button type="button" className="btn">
        Salva
      </button>,
    );
    expect(auditA11y(container)).toEqual([]);
  });

  it("segnala un bersaglio sotto 24 px", () => {
    const { container } = render(
      <button type="button" style={{ width: 20, height: 20 }}>
        Chiudi
      </button>,
    );
    expect(auditA11y(container)).toEqual(['size: button "Chiudi" 20×20']);
  });

  it("segnala controlli annidati", () => {
    const { container } = render(
      <div role="button" tabIndex={0} aria-label="Miniatura" className="btn icon-btn">
        <button type="button" className="btn">
          Apri gruppo
        </button>
      </div>,
    );
    expect(auditA11y(container)).toEqual(['nested: button "Apri gruppo" in div "Miniatura"']);
  });

  it("segnala un pulsante con sola icona senza nome", () => {
    const { container } = render(
      <button type="button" className="btn icon-btn">
        <svg aria-hidden="true" />
      </button>,
    );
    expect(auditA11y(container)).toEqual(["unnamed: button.btn.icon-btn"]);
  });

  it("ignora gli elementi nascosti e accetta i problemi noti", () => {
    const { container } = render(
      <div>
        <div hidden>
          <button type="button" style={{ height: 10 }}>
            x
          </button>
        </div>
        <button type="button" style={{ height: 12, width: 12 }} data-testid="legacy">
          y
        </button>
      </div>,
    );
    expectAccessible(container, ["size: button[data-testid=legacy]"]);
  });
});
