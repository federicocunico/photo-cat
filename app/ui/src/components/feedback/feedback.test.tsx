import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import "../../i18n";
import { QueryView } from "./QueryView";
import { BusyButton } from "./BusyButton";
import { ToastHost } from "./ToastHost";
import { notify, useToast } from "../../store/toasts";

afterEach(() => {
  cleanup();
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
