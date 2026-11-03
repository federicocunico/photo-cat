import { act, fireEvent, render, screen, within } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import "../../i18n";
import { QueryView } from "./QueryView";
import { BusyButton } from "./BusyButton";
import { ToastHost } from "./ToastHost";
import { notify, notifyError, useToast } from "../../store/toasts";
import { checkedCleanup } from "../../test/a11y";

afterEach(() => {
  checkedCleanup()();
  vi.useRealTimers();
  useToast.setState({ notices: [] });
});

it("distinguishes loading, empty and failure and retries the query", () => {
  const query = {
    isPending: true,
    isError: false,
    refetch: vi.fn(),
    data: undefined as string[] | undefined,
  };
  const view = () => (
    <QueryView query={query} empty={<p>Empty list</p>}>
      {(data) => <p>{data.join(",")}</p>}
    </QueryView>
  );
  const { rerender } = render(view());
  expect(screen.getByRole("status")).toHaveAccessibleName("Caricamento…");
  expect(screen.queryByText("Empty list")).not.toBeInTheDocument();
  query.isPending = false;
  query.isError = true;
  rerender(view());
  fireEvent.click(screen.getByRole("button", { name: "Riprova" }));
  expect(query.refetch).toHaveBeenCalledOnce();
  query.isError = false;
  query.data = [];
  rerender(view());
  expect(screen.getByText("Empty list")).toBeInTheDocument();
});

it("locks before a second click and keeps the label while awaiting", async () => {
  let finish!: () => void;
  const action = vi.fn(
    () =>
      new Promise<void>((resolve) => {
        finish = resolve;
      }),
  );
  render(<BusyButton onClick={action}>Save</BusyButton>);
  const button = screen.getByRole("button", { name: /Save/ });
  fireEvent.click(button);
  fireEvent.click(button);
  expect(action).toHaveBeenCalledOnce();
  expect(button).toHaveAttribute("aria-busy", "true");
  expect(button).toHaveTextContent("Save");
  await act(async () => finish());
  expect(button).toBeEnabled();
  expect(button).not.toHaveAttribute("aria-busy");
});

it("offers a toast action and dismisses notices automatically", async () => {
  vi.useFakeTimers();
  const undo = vi.fn();
  notify({ message: "Saved", action: { label: "Undo", run: undo } });
  render(<ToastHost />);
  await act(async () => fireEvent.click(screen.getByRole("button", { name: "Undo" })));
  expect(undo).toHaveBeenCalledOnce();
  expect(screen.queryByText("Saved")).not.toBeInTheDocument();
  act(() => {
    notify({ message: "Done", duration: 2000 });
  });
  act(() => {
    vi.advanceTimersByTime(2000);
  });
  expect(screen.queryByText("Done")).not.toBeInTheDocument();
});

describe("ToastHost", () => {
  it("impila due avvisi, ciascuno con la propria chiusura", async () => {
    render(<ToastHost />);
    act(() => {
      notify({ message: "Primo" });
      notify({ message: "Secondo", tone: "ok" });
    });
    const statuses = screen.getAllByRole("status");
    expect(statuses.map((s) => s.textContent)).toEqual(["Primo", "Secondo"]);
    fireEvent.click(within(statuses[0]).getByRole("button", { name: "Chiudi l'avviso" }));
    expect(screen.queryByText("Primo")).not.toBeInTheDocument();
    expect(screen.getByText("Secondo")).toBeInTheDocument();
  });

  it("gli errori sono annunciati come alert e non si chiudono da soli", () => {
    vi.useFakeTimers();
    render(<ToastHost />);
    act(() => {
      notifyError({ detail: "Cartella non raggiungibile" });
    });
    act(() => {
      vi.advanceTimersByTime(60_000);
    });
    expect(screen.getByRole("alert")).toHaveTextContent("Cartella non raggiungibile");
  });

  it("il tempo si ferma con il puntatore sopra", () => {
    vi.useFakeTimers();
    render(<ToastHost />);
    act(() => {
      notify({ message: "Tag assegnato", duration: 1000 });
    });
    fireEvent.pointerEnter(screen.getByRole("status"));
    act(() => {
      vi.advanceTimersByTime(5000);
    });
    expect(screen.getByText("Tag assegnato")).toBeInTheDocument();
    fireEvent.pointerLeave(screen.getByRole("status"));
    act(() => {
      vi.advanceTimersByTime(1000);
    });
    expect(screen.queryByText("Tag assegnato")).not.toBeInTheDocument();
  });

  it("un avviso con la stessa chiave sostituisce il precedente e lo segnala", () => {
    const first = vi.fn();
    render(<ToastHost />);
    act(() => {
      notify({ key: "esito", message: "Escluse 3 foto", onDismiss: first });
      notify({ key: "esito", message: "Escluse 5 foto" });
    });
    expect(screen.queryByText("Escluse 3 foto")).not.toBeInTheDocument();
    expect(screen.getByText("Escluse 5 foto")).toBeInTheDocument();
    expect(first).toHaveBeenCalledOnce();
  });
});
