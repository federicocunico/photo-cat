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
