import type { ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { Skeleton } from "./Skeleton";
import { RefreshBar } from "./RefreshBar";

export type QueryState<T> = {
  data?: T;
  isPending: boolean;
  isError: boolean;
  isFetching?: boolean;
  error?: unknown;
  refetch: () => unknown;
};

export function QueryView<T>({
  query,
  empty,
  skeleton = <Skeleton />,
  isEmpty,
  children,
  enabled = true,
  error: errorMessage,
}: {
  query: QueryState<T>;
  empty?: ReactNode;
  skeleton?: ReactNode;
  isEmpty?: (data: T) => boolean;
  children: (data: T) => ReactNode;
  enabled?: boolean;
  /** Messaggio d'errore al posto del dettaglio tecnico (il pulsante Riprova resta). */
  error?: ReactNode;
}) {
  const { t } = useTranslation();
  if (!enabled) return children(query.data as T);
  const hasData = query.data !== undefined;
  const error = query.error as { detail?: string; message?: string } | null;
  const failed = query.isError && (
    <div role="alert" className="feedback-error">
      {errorMessage ?? error?.detail ?? error?.message ?? t("feedback.error")}{" "}
      <button
        type="button"
        className="btn"
        onClick={() => {
          void query.refetch();
        }}
      >
        {t("feedback.retry")}
      </button>
    </div>
  );
  if (!hasData) {
    if (query.isError) return failed;
    if (query.isPending) return skeleton;
    return null;
  }
  const data = query.data as T;
  const nothing = isEmpty ? isEmpty(data) : Array.isArray(data) && data.length === 0;
  return (
    <>
      <RefreshBar active={!!query.isFetching && !query.isPending} />
      {failed}
      {nothing ? (empty ?? <p className="muted">{t("feedback.empty")}</p>) : children(data)}
    </>
  );
}
