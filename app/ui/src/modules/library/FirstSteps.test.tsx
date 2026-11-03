import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import "../../i18n";
import { commandsMock, ipcMockModule } from "../../test/ipcMock";
import { checkedCleanup } from "../../test/a11y";
import { A11Y_KNOWN } from "../../test/a11yKnown";
import { useGuide } from "../../store/guide";
import { useLayout } from "../../store/layout";
import { FirstSteps } from "./FirstSteps";

vi.mock("../../lib/ipc", () => ipcMockModule(true));
beforeEach(() => {
  vi.clearAllMocks();
  localStorage.removeItem("photo-cat.guide");
});
afterEach(checkedCleanup(A11Y_KNOWN.none));

const renderIt = () => {
  const qc = new QueryClient();
  render(
    <QueryClientProvider client={qc}>
      <FirstSteps />
    </QueryClientProvider>,
  );
  return qc;
};

it("Passo completato altrove: un catalogo creato da un'altra parte completa la voce", async () => {
  const qc = renderIt();
  expect(screen.getByRole("heading", { name: /Primi passi · 0 di 5/ })).toBeInTheDocument();
  expect(await screen.findByRole("button", { name: "Crea un catalogo" })).toBeInTheDocument();
  // La barra delle azioni della Libreria crea il catalogo e invalida ["catalogs"].
  commandsMock.listCatalogs.mockResolvedValue([{ catalog: { id: 1 }, items: 3 }] as never);
  await act(() => qc.invalidateQueries({ queryKey: ["catalogs"] }));
  expect(await screen.findByText("Crea un catalogo: fatto")).toBeInTheDocument();
  expect(screen.queryByRole("button", { name: "Crea un catalogo" })).toBeNull();
  expect(screen.getByRole("heading", { name: /1 di 5/ })).toBeInTheDocument();
  expect(useGuide.getState().done.catalog).toBe(true);
});

it("ogni voce porta al punto giusto", async () => {
  renderIt();
  fireEvent.click(await screen.findByRole("button", { name: "Scarica i modelli AI" }));
  expect(useLayout.getState().module).toBe("settings");
  expect(useLayout.getState().settingsSection).toBe("models");
});

it("Lista nascosta: resta nascosta dopo un riavvio e Aiuto › Primi passi la riapre", async () => {
  renderIt();
  fireEvent.click(screen.getByRole("button", { name: "Nascondi primi passi" }));
  expect(screen.queryByRole("heading", { name: /Primi passi/ })).toBeNull();
  // Riavvio: lo stato torna da localStorage (e dal file ui-prefs.json).
  const saved = localStorage.getItem("photo-cat.guide")!;
  expect(JSON.parse(saved).state.hidden).toBe(true);
  useGuide.setState({ hidden: false });
  localStorage.setItem("photo-cat.guide", saved);
  await act(() => useGuide.persist.rehydrate());
  expect(useGuide.getState().hidden).toBe(true);
  act(() => useGuide.getState().show());
  expect(screen.getByRole("heading", { name: /Primi passi/ })).toBeInTheDocument();
});

it("sparisce quando tutte le voci sono completate", async () => {
  useGuide.setState({
    done: { import: true, models: true, person: true, catalog: true, tags: true },
  });
  renderIt();
  await waitFor(() => expect(screen.queryByRole("heading", { name: /Primi passi/ })).toBeNull());
});
