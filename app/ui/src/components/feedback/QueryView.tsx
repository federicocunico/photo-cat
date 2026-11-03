import type { ReactNode } from "react";

/**
 * Stati caricamento / errore / vuoto / contenuto per query TanStack.
 * Placeholder locale finché `loading-feedback` non unisce il componente condiviso.
 */
export function QueryView({
  isLoading,
  isError,
  isEmpty,
  loading,
  error,
  empty,
  children,
}: {
  isLoading: boolean;
  isError: boolean;
  isEmpty: boolean;
  loading: ReactNode;
  error: ReactNode;
  empty: ReactNode;
  children: ReactNode;
}) {
  if (isLoading) return <>{loading}</>;
  if (isError) return <>{error}</>;
  if (isEmpty) return <>{empty}</>;
  return <>{children}</>;
}
