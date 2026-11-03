import { useRef, useState } from "react";

/** Also guards keyboard/form submissions and actions shared by several buttons. */
export function useBusyAction<A extends unknown[], T>(action: (...args: A) => Promise<T>) {
  const lock = useRef(false);
  const [busy, setBusy] = useState(false);
  const run = async (...args: A) => {
    if (lock.current) return;
    lock.current = true;
    setBusy(true);
    try {
      return await action(...args);
    } finally {
      lock.current = false;
      setBusy(false);
    }
  };
  return [run, busy] as const;
}
