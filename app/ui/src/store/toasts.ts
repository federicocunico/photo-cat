import { create } from "zustand";

export type Notice = {
  id: number;
  message: string;
  duration: number;
  action?: { label: string; run: () => unknown };
};
let next = 0;
export const useToast = create<{ notices: Notice[]; dismiss: (id: number) => void }>((set) => ({
  notices: [],
  dismiss: (id) => set((s) => ({ notices: s.notices.filter((n) => n.id !== id) })),
}));
export function notify(input: Omit<Notice, "id" | "duration"> & { duration?: number }) {
  const notice = { ...input, id: ++next, duration: input.duration ?? 8000 };
  useToast.setState((s) => ({ notices: [...s.notices, notice].slice(-5) }));
  return notice.id;
}
