import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { act, cleanup, fireEvent, render as rtlRender, screen } from "@testing-library/react";
import type { ReactElement } from "react";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import "../../i18n";
import { ipcMockModule } from "../../test/ipcMock";
import { useLayout } from "../../store/layout";
import { AiPage } from "./AiPage";

vi.mock("../../lib/ipc", () => ipcMockModule(true));

afterEach(cleanup);

// Le sezioni leggono i dati con react-query.
function render(ui: ReactElement) {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return rtlRender(<QueryClientProvider client={client}>{ui}</QueryClientProvider>);
}

beforeEach(() => {
  useLayout.setState({ module: "library", aiSection: "overview" });
});

it("la prima apertura mostra Panoramica; la sotto-navigazione ha Mappa", () => {
  render(<AiPage />);
  expect(screen.getByRole("heading", { level: 2, name: "Panoramica" })).toBeInTheDocument();
  expect(screen.getByRole("button", { name: "Panoramica" })).toHaveAttribute("aria-current", "page");
  expect(screen.getByRole("button", { name: "Mappa" })).toBeInTheDocument();
  for (const name of ["Rianalisi", "Risultati", "Pulizia", "Simili"])
    expect(screen.getByRole("button", { name })).toBeInTheDocument();
});

it("una destinazione con sezione apre quella sezione", () => {
  render(<AiPage />);
  act(() => useLayout.getState().openModule({ module: "ai", section: "reprocess" }));
  expect(useLayout.getState().module).toBe("ai");
  expect(screen.getByRole("heading", { level: 2, name: "Rianalisi" })).toBeInTheDocument();
});

it("la sezione scelta si ricorda; senza sezione resta quella di prima", () => {
  const { unmount } = render(<AiPage />);
  fireEvent.click(screen.getByRole("button", { name: "Simili" }));
  expect(useLayout.getState().aiSection).toBe("similar");
  unmount();
  act(() => useLayout.getState().openModule({ module: "ai" }));
  render(<AiPage />);
  expect(screen.getByRole("heading", { level: 2, name: "Simili" })).toBeInTheDocument();
});
