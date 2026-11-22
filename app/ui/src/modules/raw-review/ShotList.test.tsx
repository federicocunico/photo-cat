import { useState } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { act, cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import "../../i18n";
import type { ReviewShotView, ShotDecision } from "../../bindings";
import { useGridShortcuts } from "../../commands/gridShortcuts";
import { useCommandStore } from "../../commands/registry";
import { commandsMock, ipcMockModule } from "../../test/ipcMock";
import { ShotList } from "./ShotList";

vi.mock("../../lib/ipc", () => ipcMockModule(true));
vi.mock("@tauri-apps/api/core", () => ({
  convertFileSrc: (key: string, scheme: string) => `${scheme}://${key}`,
}));

const shots = (n: number): ReviewShotView[] =>
  Array.from({ length: n }, (_, i) => {
    const key = `dscf${String(i + 1).padStart(4, "0")}`;
    return {
      key,
      displayName: key.toUpperCase(),
      variants: [
        {
          preset: "Acros",
          fileName: `${key}.jpg`,
          extension: "jpg",
          bytes: 1,
          kind: "jpeg",
          selectable: true,
          exportable: true,
        },
      ],
    } as unknown as ReviewShotView;
  });

function Harness({
  list,
  decisions,
  onSelectionChange,
}: {
  list: ReviewShotView[];
  decisions?: Record<string, ShotDecision>;
  onSelectionChange?: (keys: string[]) => void;
}) {
  useGridShortcuts();
  const [current, setCurrent] = useState(0);
  return (
    <ShotList
      sessionId="s1"
      shots={list}
      decisions={decisions}
      current={current}
      onOpen={setCurrent}
      onSelectionChange={onSelectionChange}
    />
  );
}

beforeEach(() => {
  vi.clearAllMocks();
  useCommandStore.setState({ commands: [] });
  commandsMock.rawReviewThumbnails.mockImplementation(async (_id: string, keys: string[]) =>
    keys.map((shotKey) => ({ shotKey, key: `thumb-${shotKey}` })),
  );
});
afterEach(cleanup);

describe("elenco degli scatti", () => {
  it("è virtualizzato e chiede le miniature solo per le righe visibili", async () => {
    render(<Harness list={shots(160)} />);
    const rows = screen.getAllByRole("option");
    expect(rows.length).toBeGreaterThan(5);
    expect(rows.length).toBeLessThan(40);
    await waitFor(() => expect(commandsMock.rawReviewThumbnails).toHaveBeenCalled());
    const asked = commandsMock.rawReviewThumbnails.mock.calls.flatMap((c) => c[1]);
    expect(asked).toContain("dscf0001");
    expect(asked).not.toContain("dscf0160");
    expect(asked.length).toBeLessThan(40);
    expect(commandsMock.rawReviewThumbnails.mock.calls[0][0]).toBe("s1");
    await waitFor(() =>
      expect(rows[0].querySelector("img")).toHaveAttribute("src", "review://thumb-dscf0001"),
    );
  });

  it("mostra nome, stelle, variante scelta e stato di revisione", () => {
    render(
      <Harness
        list={shots(3)}
        decisions={{
          dscf0001: { key: "dscf0001", rating: 4, winnerPreset: "Velvia", reviewed: true },
          dscf0002: { key: "dscf0002", rating: 0, winnerPreset: null, reviewed: true },
        }}
      />,
    );
    const first = screen.getByRole("option", { name: /DSCF0001/ });
    expect(first).toHaveAccessibleName("DSCF0001, 4 stelle, variante migliore Velvia, rivisto");
    expect(within(first).getByText("Velvia")).toBeInTheDocument();
    expect(screen.getByRole("option", { name: "DSCF0002, rivisto" })).toBeInTheDocument();
    expect(screen.getByRole("option", { name: "DSCF0003" })).toBeInTheDocument();
  });

  it("il clic apre lo scatto e lo seleziona; Ctrl e Maiusc compongono la selezione", () => {
    const onSelectionChange = vi.fn();
    render(<Harness list={shots(10)} onSelectionChange={onSelectionChange} />);
    fireEvent.click(screen.getByRole("option", { name: "DSCF0003" }));
    expect(screen.getByRole("option", { name: "DSCF0003" })).toHaveAttribute(
      "aria-current",
      "true",
    );
    expect(onSelectionChange).toHaveBeenLastCalledWith(["dscf0003"]);
    fireEvent.click(screen.getByRole("option", { name: "DSCF0006" }), { shiftKey: true });
    expect(onSelectionChange).toHaveBeenLastCalledWith([
      "dscf0003",
      "dscf0004",
      "dscf0005",
      "dscf0006",
    ]);
    // Maiusc+clic non cambia lo scatto mostrato.
    expect(screen.getByRole("option", { name: "DSCF0003" })).toHaveAttribute(
      "aria-current",
      "true",
    );
    fireEvent.click(screen.getByRole("option", { name: "DSCF0008" }), { ctrlKey: true });
    expect(screen.getByText("5 selezionati")).toBeInTheDocument();
  });

  it("Ctrl+A nella lista seleziona tutti gli scatti, Esc li toglie", async () => {
    const onSelectionChange = vi.fn();
    render(<Harness list={shots(160)} onSelectionChange={onSelectionChange} />);
    // Senza il fuoco nella lista, Ctrl+A non fa nulla.
    fireEvent.keyDown(window, { key: "a", ctrlKey: true });
    expect(screen.queryByText(/selezionati/)).not.toBeInTheDocument();

    act(() => screen.getByRole("listbox", { name: "Elenco degli scatti" }).focus());
    fireEvent.keyDown(window, { key: "a", ctrlKey: true });
    expect(await screen.findByText("160 selezionati")).toBeInTheDocument();
    expect(onSelectionChange.mock.lastCall?.[0]).toHaveLength(160);

    fireEvent.keyDown(window, { key: "i", ctrlKey: true, shiftKey: true });
    await waitFor(() => expect(screen.queryByText(/selezionati/)).not.toBeInTheDocument());

    fireEvent.keyDown(window, { key: "a", ctrlKey: true });
    await screen.findByText("160 selezionati");
    fireEvent.keyDown(window, { key: "Escape" });
    await waitFor(() => expect(screen.queryByText(/selezionati/)).not.toBeInTheDocument());
  });

  it("le frecce nell'elenco cambiano lo scatto", () => {
    render(<Harness list={shots(5)} />);
    const list = screen.getByRole("listbox", { name: "Elenco degli scatti" });
    act(() => list.focus());
    fireEvent.keyDown(list, { key: "ArrowDown" });
    fireEvent.keyDown(list, { key: "ArrowDown" });
    expect(screen.getByRole("option", { name: "DSCF0003" })).toHaveAttribute(
      "aria-current",
      "true",
    );
    expect(list).toHaveAttribute("aria-activedescendant", "rr-shot-2");
    fireEvent.keyDown(list, { key: "End" });
    expect(list).toHaveAttribute("aria-activedescendant", "rr-shot-4");
  });
});
