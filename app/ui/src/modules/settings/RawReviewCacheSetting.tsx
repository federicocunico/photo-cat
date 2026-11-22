// Impostazioni › Generale: limite della cache delle anteprime di Varianti RAW, spazio in uso
// e "Svuota la cache". Il limite è una preferenza (salvata subito) e il motore lo applica a
// caldo, eliminando le anteprime usate meno di recente.
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useId, useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "../../components/ui";
import { commands, isTauri } from "../../lib/ipc";
import { fmtBytes } from "../../lib/format";
import { notify, notifyError } from "../../store/toasts";
import { Setting } from "./settingsState";
import { useSettings } from "./settingsContext";

const MB_PER_GB = 1024;
const QUERY_KEY = ["raw-review-cache"];

export function RawReviewCacheSetting() {
  const { t } = useTranslation();
  const qc = useQueryClient();
  const { saved, autosave } = useSettings();
  const id = useId();
  const [invalid, setInvalid] = useState(false);
  const [clearing, setClearing] = useState(false);
  const info = useQuery({
    queryKey: QUERY_KEY,
    queryFn: commands.rawReviewCacheInfo,
    enabled: isTauri(),
  });
  const limitMb = saved.resources.raw_review_cache_mb ?? null;
  const refresh = () => void qc.invalidateQueries({ queryKey: QUERY_KEY });

  const clear = async () => {
    setClearing(true);
    try {
      const freed = await commands.rawReviewClearCache();
      notify({
        tone: "ok",
        message: t("settings.rawReviewCache.cleared", { size: fmtBytes(freed) }),
      });
    } catch (e) {
      notifyError(e);
    } finally {
      setClearing(false);
      refresh();
    }
  };

  const usage = info.data
    ? info.data.available
      ? t("settings.rawReviewCache.used", {
          used: fmtBytes(info.data.usedBytes),
          limit: fmtBytes(info.data.limitBytes),
        })
      : t("settings.rawReviewCache.unavailable")
    : null;

  return (
    <Setting
      id="rawReviewCache"
      label={t("settings.rawReviewCache.label")}
      labelFor={id}
      hint={
        <>
          {usage && (
            <>
              <span data-testid="raw-review-cache-usage">{usage}</span>
              <br />
            </>
          )}
          {invalid ? t("settings.rawReviewCache.invalid") : t("settings.rawReviewCache.hint")}
        </>
      }
    >
      <input
        id={id}
        className="field"
        type="number"
        min={1}
        step={1}
        placeholder="20"
        aria-invalid={invalid || undefined}
        defaultValue={limitMb == null ? "" : limitMb / MB_PER_GB}
        onChange={(e) => {
          const text = e.target.value.trim();
          const gb = Number(text);
          // Il motore rifiuta i limiti troppo bassi: si aspetta un valore valido.
          const ok = text === "" || (Number.isFinite(gb) && gb >= 1);
          setInvalid(!ok);
          if (!ok) return;
          const raw_review_cache_mb = text === "" ? null : Math.round(gb * MB_PER_GB);
          autosave(
            "rawReviewCache",
            (c) => ({ ...c, resources: { ...c.resources, raw_review_cache_mb } }),
            refresh,
          );
        }}
      />
      <Button
        onClick={() => void clear()}
        disabled={clearing || !info.data?.available || info.data.usedBytes === 0}
      >
        {t("settings.rawReviewCache.clear")}
      </Button>
    </Setting>
  );
}
