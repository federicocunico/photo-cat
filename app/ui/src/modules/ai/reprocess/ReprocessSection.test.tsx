import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import "../../../i18n";
import { commandsMock, ipcMockModule } from "../../../test/ipcMock";
import { useLibrary, idsSelection } from "../../../store/library";
import { useLayout } from "../../../store/layout";
import { ReprocessSection } from "../ReprocessSection";
import { openReprocessFor } from "./openReprocess";
import { useReprocessPreset } from "./preset";

vi.mock("../../../lib/ipc", () => ipcMockModule(true));

const mocks = commandsMock as unknown as Record<string, ReturnType<typeof vi.fn>>;

const wrap = () =>
  render(
    <QueryClientProvider client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}>
      <ReprocessSection />
    </QueryClientProvider>,
  );

const cat = (id: number, name: string) => ({
  catalog: { id, name, kind: "manual", rules: null, createdAt: 0 },
  items: 3,
  excluded: 0,
});

let counts: Array<{ stage: string; count: number; available: boolean }>;

beforeEach(() => {
  vi.clearAllMocks();
  useReprocessPreset.getState().clear();
  useLibrary.getState().setSelection([], null);
  counts = [
    { stage: "classification", count: 5, available: true },
    { stage: "persons", count: 5, available: true },
    { stage: "faces", count: 5, available: true },
    { stage: "similar", count: 5, available: true },
    { stage: "phash", count: 0, available: false },
  ];
  mocks.listCatalogs = vi.fn(async () => [cat(1, "Vacanze"), cat(2, "Matrimonio")]);
  mocks.aiReprocessPreview = vi.fn(async () => counts);
  mocks.aiReprocess = vi.fn(async () => 42);
});
afterEach(cleanup);

it("due cataloghi, solo mancanti: anteprima e avvio con i parametri scelti", async () => {
  wrap();
  fireEvent.click(screen.getByRole("radio", { name: "Cataloghi" }));
  fireEvent.click(await screen.findByRole("checkbox", { name: "Vacanze" }));
  fireEvent.click(screen.getByRole("checkbox", { name: "Matrimonio" }));
  await waitFor(() =>
    expect(mocks.aiReprocessPreview).toHaveBeenCalledWith(
      { kind: "catalogs", ids: [1, 2] },
      ["classification", "persons", "faces", "similar"],
      false,
    ),
  );
  const start = screen.getByRole("button", { name: "Avvia rianalisi" });
  await waitFor(() => expect(start).toBeEnabled());
  fireEvent.click(start);
  await waitFor(() =>
    expect(mocks.aiReprocess).toHaveBeenCalledWith(
      { kind: "catalogs", ids: [1, 2] },
      ["classification", "persons", "faces", "similar"],
      false,
    ),
  );
});

it("ambito vuoto: avvio disattivato", async () => {
  counts = counts.map((c) => ({ ...c, count: 0 }));
  wrap();
  await screen.findByText("0 foto da elaborare");
  expect(screen.getByRole("button", { name: "Avvia rianalisi" })).toBeDisabled();
});

it("catalogo non scelto: nessuna anteprima e avvio disattivato", async () => {
  wrap();
  fireEvent.click(screen.getByRole("radio", { name: "Cataloghi" }));
  expect(screen.getByRole("button", { name: "Avvia rianalisi" })).toBeDisabled();
});

it("forza sulla selezione: bersaglio e sole fasi scelte", async () => {
  useLibrary.getState().setSelection([1, 2, 3], 1);
  wrap();
  fireEvent.click(screen.getByRole("radio", { name: /Selezione corrente/ }));
  fireEvent.click(screen.getByRole("checkbox", { name: "Classificazione" }));
  fireEvent.click(screen.getByRole("checkbox", { name: "Gruppi di simili" }));
  fireEvent.click(screen.getByRole("radio", { name: "Forza" }));
  const target = { kind: "ids", ids: [1, 2, 3] };
  await waitFor(() =>
    expect(mocks.aiReprocessPreview).toHaveBeenLastCalledWith(
      { kind: "target", target },
      ["persons", "faces"],
      true,
    ),
  );
  const start = screen.getByRole("button", { name: "Avvia rianalisi" });
  await waitFor(() => expect(start).toBeEnabled());
  fireEvent.click(start);
  await waitFor(() =>
    expect(mocks.aiReprocess).toHaveBeenCalledWith(
      { kind: "target", target },
      ["persons", "faces"],
      true,
    ),
  );
});

it("fase non disponibile: grigia con avviso", async () => {
  wrap();
  await screen.findByText("modello non installato");
  expect(screen.getByRole("checkbox", { name: "Impronte" })).toBeDisabled();
});

it("Rianalizza dalla Libreria apre la sezione con la selezione precompilata", async () => {
  useLibrary.setState({ selection: idsSelection([7, 8]) });
  openReprocessFor({ kind: "ids", ids: [7, 8] }, 2);
  expect(useLayout.getState().module).toBe("ai");
  expect(useLayout.getState().aiSection).toBe("reprocess");
  wrap();
  expect(screen.getByRole("radio", { name: /Selezione corrente/ })).toBeChecked();
  await waitFor(() =>
    expect(mocks.aiReprocessPreview).toHaveBeenCalledWith(
      { kind: "target", target: { kind: "ids", ids: [7, 8] } },
      expect.anything(),
      false,
    ),
  );
  expect(useReprocessPreset.getState().scope).toBeNull();
});
