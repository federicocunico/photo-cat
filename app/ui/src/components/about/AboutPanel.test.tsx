// Informazioni su photo-cat (app-about; ui-usability-audit 5.6).
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import "../../i18n";
import { commandsMock, ipcMockModule } from "../../test/ipcMock";
import { checkedCleanup } from "../../test/a11y";
import { A11Y_KNOWN } from "../../test/a11yKnown";
import { AboutPanel } from "./AboutPanel";

vi.mock("../../lib/ipc", () => ipcMockModule(true));
const writeText = vi.fn<(text: string) => Promise<void>>(async () => {});
beforeEach(() => {
  writeText.mockClear();
  Object.defineProperty(navigator, "clipboard", { value: { writeText }, configurable: true });
});
afterEach(checkedCleanup(A11Y_KNOWN.none));

const renderPanel = () =>
  render(
    <QueryClientProvider client={new QueryClient()}>
      <AboutPanel />
    </QueryClientProvider>,
  );

describe("scheda Informazioni", () => {
  it("Versione esplicita: nome, versione, build con commit, data e configurazione", async () => {
    renderPanel();
    expect(await screen.findByText("photo-cat 0.5.1")).toBeInTheDocument();
    expect(screen.getByText(/^a1b2c3d · .+ · release$/)).toBeInTheDocument();
    expect(screen.getByText("Windows 11 Pro x86_64")).toBeInTheDocument();
    expect(screen.getByText("DirectML")).toBeInTheDocument();
    expect(screen.getByText("photo-cat contributors")).toBeInTheDocument();
    expect(screen.getByText("MIT oppure Apache-2.0")).toBeInTheDocument();
  });

  it("Build senza repository git: «commit non disponibile» con data e configurazione", async () => {
    const info = await commandsMock.appInfo();
    commandsMock.appInfo.mockResolvedValueOnce({ ...info, commit: null });
    renderPanel();
    expect(await screen.findByText(/^commit non disponibile · .+ · release$/)).toBeInTheDocument();
  });

  it("Aprire la cartella dei dati", async () => {
    renderPanel();
    fireEvent.click(await screen.findByRole("button", { name: "Apri" }));
    await waitFor(() => expect(commandsMock.openDataDir).toHaveBeenCalled());
  });

  it("Copia informazioni: una riga per voce, senza il percorso dei dati", async () => {
    renderPanel();
    fireEvent.click(await screen.findByRole("button", { name: "Copia informazioni" }));
    await waitFor(() => expect(writeText).toHaveBeenCalledTimes(1));
    const text = writeText.mock.calls[0]![0];
    expect(text.split("\n")).toEqual([
      "Applicazione: photo-cat 0.5.1",
      expect.stringMatching(/^Build: a1b2c3d · .+ · release$/),
      "Sistema: Windows 11 Pro x86_64",
      "Motore della finestra: 131.0.2903.70",
      "Acceleratore AI: DirectML",
    ]);
    expect(text).not.toContain("D:/photo-cat");
  });

  it("se gli appunti non sono disponibili lo dice", async () => {
    writeText.mockRejectedValueOnce(new Error("negato"));
    renderPanel();
    fireEvent.click(await screen.findByRole("button", { name: "Copia informazioni" }));
    expect(await screen.findByText(/Copia non riuscita/)).toBeInTheDocument();
  });
});

describe("scheda Licenze di terze parti", () => {
  it("Ricerca di una licenza: «LGPL» mostra libheif e libde265 con testo e sorgenti", async () => {
    renderPanel();
    const user = userEvent.setup();
    await user.click(screen.getByRole("tab", { name: "Licenze di terze parti" }));
    const search = await screen.findByRole("searchbox", {
      name: "Cerca un componente o una licenza",
    });
    await user.type(search, "LGPL");
    const native = await screen.findByRole("region", { name: "Librerie native" });
    const rows = within(native).getAllByRole("button");
    expect(rows.map((r) => r.textContent)).toEqual([
      expect.stringContaining("libheif"),
      expect.stringContaining("libde265"),
    ]);
    expect(screen.queryByRole("region", { name: "Componenti Rust" })).toBeNull();
    await user.click(rows[0]!);
    expect(rows[0]).toHaveAttribute("aria-expanded", "true");
    expect(screen.getByText("GNU LESSER GENERAL PUBLIC LICENSE")).toBeInTheDocument();
    expect(
      screen.getByText("C:\\Program Files\\photo-cat\\native-sources\\libheif-1.23.7.tar.gz"),
    ).toBeInTheDocument();
  });
});
