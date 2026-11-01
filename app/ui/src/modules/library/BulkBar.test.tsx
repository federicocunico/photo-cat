import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, it, expect, vi } from "vitest";
import "../../i18n";
import { commandsMock, ipcMockModule } from "../../test/ipcMock";
import { emptyFilter, rectangleSelect, useLibrary } from "../../store/library";
import { BulkBar } from "./BulkBar";
vi.mock("../../lib/ipc", () => ipcMockModule(true));
afterEach(cleanup);
it("selects unseen filtered results, inverts and confirms destructive edits", async () => {
  useLibrary.setState({ filter: emptyFilter(), selected: [] });
  commandsMock.libraryIds.mockResolvedValue(Array.from({ length: 2340 }, (_, i) => i + 1));
  const confirm = vi.spyOn(window, "confirm").mockReturnValue(false);
  render(
    <QueryClientProvider client={new QueryClient()}>
      <BulkBar />
    </QueryClientProvider>,
  );
  fireEvent.click(screen.getByText("Seleziona tutto (filtro)"));
  await waitFor(() => expect(useLibrary.getState().selected).toHaveLength(2340));
  fireEvent.click(screen.getByText("Inverti"));
  await waitFor(() => expect(useLibrary.getState().selected).toHaveLength(0));
  useLibrary.setState({ selected: [1, 2] });
  fireEvent.change(screen.getByLabelText("Azione"), { target: { value: "remove_library" } });
  fireEvent.click(screen.getByText("Applica"));
  await waitFor(() => expect(confirm).toHaveBeenCalledWith(expect.stringContaining("2 foto")));
  expect(commandsMock.startBulk).not.toHaveBeenCalled();
  confirm.mockRestore();
});
it("rectangle selects rows independently of the virtual DOM", () => {
  const ids = Array.from({ length: 100000 }, (_, i) => i + 1);
  expect(
    rectangleSelect(
      ids,
      { x: 16, y: 216, w: 200, h: 99 },
      { cols: 4, cell: 90, rowHeight: 100, gap: 10, pad: 16 },
    ),
  ).toEqual([9, 10]);
});
it("imposta e ripristina il luogo sulla selezione", async () => {
  vi.clearAllMocks();
  useLibrary.setState({ filter: emptyFilter(), selected: [4, 5, 6] });
  render(
    <QueryClientProvider client={new QueryClient()}>
      <BulkBar />
    </QueryClientProvider>,
  );
  fireEvent.change(screen.getByLabelText("Azione"), { target: { value: "set_place" } });
  expect(screen.getByText("Applica")).toBeDisabled();
  fireEvent.change(screen.getByLabelText("Valore azione"), { target: { value: " Giappone " } });
  fireEvent.click(screen.getByText("Applica"));
  await waitFor(() =>
    expect(commandsMock.startBulk).toHaveBeenCalledWith(
      [4, 5, 6],
      { action: "set_place", place: { name: "Giappone" } },
      false,
    ),
  );
  fireEvent.change(screen.getByLabelText("Azione"), { target: { value: "reset_place" } });
  fireEvent.click(screen.getByText("Applica"));
  await waitFor(() =>
    expect(commandsMock.startBulk).toHaveBeenLastCalledWith(
      [4, 5, 6],
      { action: "set_place", place: null },
      false,
    ),
  );
});
it("nuova voce in bulk: creata e assegnata alle 60 foto selezionate", async () => {
  vi.clearAllMocks();
  const ids = Array.from({ length: 60 }, (_, i) => i + 1);
  useLibrary.setState({ filter: emptyFilter(), selected: ids });
  commandsMock.getTaxonomy.mockResolvedValue({
    version: 4,
    groups: [
      {
        id: "subject",
        label: "Tipo soggetto",
        threshold: 0.25,
        categories: [{ id: "fiori", label: "Fiori", enabled: true, prompts: ["flowers"] }],
      },
      {
        id: "evento",
        label: "Evento",
        threshold: 0.25,
        categories: [{ id: "laurea", label: "Laurea", enabled: true, prompts: [] }],
      },
    ],
  } as never);
  commandsMock.addCategory.mockResolvedValue({
    group: "evento",
    id: "gita_scolastica",
    label: "Gita scolastica",
    created: true,
    job: 12,
  } as never);
  render(
    <QueryClientProvider client={new QueryClient()}>
      <BulkBar />
    </QueryClientProvider>,
  );
  fireEvent.change(screen.getByLabelText("Azione"), { target: { value: "set_label" } });
  await screen.findByRole("option", { name: "Evento" });
  fireEvent.change(screen.getByLabelText("Campo"), { target: { value: "evento" } });
  expect(screen.getByRole("option", { name: "Laurea" })).toBeInTheDocument();
  fireEvent.change(screen.getByLabelText("Valore azione"), { target: { value: "__new__" } });
  fireEvent.click(screen.getByText("Conferma"));
  expect(await screen.findByRole("alert")).toHaveTextContent("Il nome è obbligatorio");
  fireEvent.change(screen.getByLabelText("Nome della nuova voce"), {
    target: { value: "Gita scolastica" },
  });
  fireEvent.click(screen.getByText("Conferma"));
  await waitFor(() =>
    expect(commandsMock.addCategory).toHaveBeenCalledWith("evento", "Gita scolastica", [], ids),
  );
  expect(commandsMock.addCategory).toHaveBeenCalledTimes(1);
  expect(commandsMock.startBulk).not.toHaveBeenCalled();
});
