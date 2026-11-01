import { useEffect, useRef, useState, type ReactNode } from "react";
import { useQuery } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { commands, isTauri, type SequenceExportOptions } from "../../lib/ipc";
import { fmtInt } from "../../lib/format";
import { useLayout } from "../../store/layout";
import { useSequence } from "../../store/sequence";
import { useTagOverview } from "../tags/data";

const TOKENS: Array<[string, string]> = [
  ["{seq}", "seq"],
  ["{seq:04d}", "seq4"],
  ["{date}", "date"],
  ["{time}", "time"],
  ["{time_full}", "timeFull"],
  ["{datetime}", "datetime"],
  ["{tag}", "tag"],
  ["{device}", "device"],
  ["{people}", "people"],
  ["{place}", "place"],
  ["{orig}", "orig"],
];

const errorText = (e: unknown) => (e as { detail?: string }).detail ?? String(e);
const gb = (b: number) =>
  `${new Intl.NumberFormat(undefined, { maximumFractionDigits: 1 }).format(b / 1e9)} GB`;

/** Valore che segue `value` con un piccolo ritardo (anteprima mentre si scrive). */
function useDebounced<T>(value: T, ms = 150): T {
  const [v, setV] = useState(value);
  useEffect(() => {
    const t = setTimeout(() => setV(value), ms);
    return () => clearTimeout(t);
  }, [value, ms]);
  return v;
}

function Section({ title, children }: { title: string; children: ReactNode }) {
  return (
    <section className="feature-card" style={{ alignItems: "stretch", gap: 10 }} aria-label={title}>
      <h2 style={{ margin: 0, fontSize: 16 }}>{title}</h2>
      {children}
    </section>
  );
}

export function SequenceExport() {
  const { t } = useTranslation();
  const scope = useSequence((s) => s.scope);
  const setView = useSequence((s) => s.setView);
  const tags = useTagOverview().data;
  const inputRef = useRef<HTMLInputElement>(null);
  const [opts, setOpts] = useState<SequenceExportOptions | null>(null);
  const [started, setStarted] = useState<number | null>(null);
  const [error, setError] = useState("");
  const defaults = useQuery({
    queryKey: ["sequence-export-defaults"],
    queryFn: commands.sequenceExportDefaults,
    enabled: isTauri(),
  });
  useEffect(() => {
    if (defaults.data && !opts)
      setOpts({
        scope,
        template: defaults.data.template,
        destination: "",
        format: "jpeg",
        jpeg: { quality: 97, chroma444: true, verify: true, minSsim: 0.985, minPsnr: 42 },
        perTag: true,
        manifest: true,
        zip: true,
        zipPerTag: true,
        metadata: true,
        tagSet: defaults.data.tagSet,
      });
  }, [defaults.data, opts, scope]);
  const debounced = useDebounced(opts);
  const preview = useQuery({
    queryKey: [
      "sequence-export-preview",
      debounced?.template,
      debounced?.tagSet,
      debounced?.format,
      scope,
    ],
    queryFn: () => commands.sequenceExportPreview(debounced!, 5),
    enabled: isTauri() && !!debounced,
  });
  const preflight = useQuery({
    queryKey: ["sequence-export-preflight", debounced],
    queryFn: () => commands.sequenceExportPreflight(debounced!),
    enabled: isTauri() && !!debounced?.destination,
    retry: false,
  });
  if (!opts)
    return (
      <p className="muted" style={{ padding: 24 }}>
        …
      </p>
    );
  const set = (p: Partial<SequenceExportOptions>) => setOpts({ ...opts, ...p });
  const jpeg = opts.jpeg ?? {};
  const errors = preview.data?.errors ?? [];
  const pre = preflight.data;
  const tagSet = tags?.sets.find((s) => s.id === opts.tagSet);
  const insert = (token: string) => {
    const el = inputRef.current;
    const [a, b] = el
      ? [el.selectionStart ?? opts.template.length, el.selectionEnd ?? opts.template.length]
      : [opts.template.length, opts.template.length];
    set({ template: opts.template.slice(0, a) + token + opts.template.slice(b) });
    requestAnimationFrame(() => {
      el?.focus();
      el?.setSelectionRange(a + token.length, a + token.length);
    });
  };
  const browse = async () => {
    const { open } = await import("@tauri-apps/plugin-dialog");
    const path = await open({ directory: true, multiple: false });
    if (typeof path === "string") set({ destination: path });
  };
  const canStart =
    !errors.length &&
    !!opts.destination &&
    !!pre?.canStart &&
    !preflight.isFetching &&
    started === null;
  const start = async () => {
    setError("");
    try {
      setStarted(await commands.startSequenceExport(opts));
    } catch (e) {
      setError(errorText(e));
    }
  };
  const check = (
    key: keyof SequenceExportOptions,
    label: string,
    help: string,
    disabled = false,
  ) => (
    <label style={{ display: "flex", gap: 8, alignItems: "flex-start" }}>
      <input
        type="checkbox"
        checked={Boolean(opts[key])}
        disabled={disabled}
        onChange={(e) => set({ [key]: e.target.checked } as Partial<SequenceExportOptions>)}
      />
      <span>
        <span className="mono">{label}</span>
        <br />
        <span className="muted" style={{ fontSize: 12 }}>
          {help}
        </span>
      </span>
    </label>
  );
  return (
    <div
      style={{
        flex: 1,
        overflow: "auto",
        padding: 24,
        display: "flex",
        flexDirection: "column",
        gap: 16,
      }}
    >
      <div style={{ display: "flex", flexWrap: "wrap", alignItems: "baseline", gap: 12 }}>
        <button className="btn btn-ghost" onClick={() => setView("sequence")}>
          ‹ {t("modules.sequence")}
        </button>
        <h1 style={{ margin: 0, fontSize: 26, fontWeight: 600 }}>{t("seqExport.title")}</h1>
        {pre && (
          <span className="muted">
            {t("seqExport.summary", {
              count: pre.count,
              tagged: fmtInt(pre.count - pre.untagged),
            })}
          </span>
        )}
      </div>
      <div style={{ display: "flex", flexWrap: "wrap", gap: 16, alignItems: "flex-start" }}>
        <div
          style={{
            flex: "2 1 520px",
            display: "flex",
            flexDirection: "column",
            gap: 16,
            minWidth: 0,
          }}
        >
          <Section title={t("seqExport.names")}>
            <label htmlFor="pc-seq-pattern" style={{ fontWeight: 600 }}>
              {t("seqExport.template")}
            </label>
            <input
              id="pc-seq-pattern"
              ref={inputRef}
              className="field mono"
              value={opts.template}
              aria-invalid={errors.length > 0}
              onChange={(e) => set({ template: e.target.value })}
              style={{ minHeight: 38 }}
            />
            <div style={{ display: "flex", flexWrap: "wrap", gap: 6 }}>
              {TOKENS.map(([tok, key]) => (
                <button
                  key={tok}
                  type="button"
                  className="chip mono"
                  style={{ border: 0, cursor: "pointer" }}
                  title={t(`seqExport.tokens.${key}`)}
                  aria-label={t("seqExport.insertToken", {
                    token: tok,
                    help: t(`seqExport.tokens.${key}`),
                  })}
                  onClick={() => insert(tok)}
                >
                  {tok}
                </button>
              ))}
            </div>
            {errors.length > 0 && (
              <p role="alert" style={{ margin: 0, color: "var(--pc-danger)" }}>
                {errors
                  .map((e) =>
                    e.kind === "unknown_token"
                      ? t("seqExport.unknownToken", { token: e.text })
                      : e.kind === "invalid_char"
                        ? t("seqExport.invalidChar", { char: e.text })
                        : e.kind === "unclosed"
                          ? t("seqExport.unclosed")
                          : t("seqExport.emptyTemplate"),
                  )
                  .join(" · ")}
              </p>
            )}
            <table className="table" aria-label={t("seqExport.preview")}>
              <thead>
                <tr>
                  <th>{t("seqExport.original")}</th>
                  <th aria-hidden="true" />
                  <th>{t("seqExport.preview")}</th>
                </tr>
              </thead>
              <tbody>
                {(preview.data?.names ?? []).map((p) => (
                  <tr key={p.original + p.name}>
                    <td className="mono muted">{p.original}</td>
                    <td aria-hidden="true">→</td>
                    <td className="mono">{p.name}</td>
                  </tr>
                ))}
              </tbody>
            </table>
            <label style={{ display: "flex", alignItems: "center", gap: 8 }}>
              {t("seqExport.tagSet")}
              <select
                className="field"
                value={opts.tagSet ?? ""}
                onChange={(e) => set({ tagSet: e.target.value ? Number(e.target.value) : null })}
              >
                <option value="">{t("seqExport.noTagSet")}</option>
                {tags?.sets.map((s) => (
                  <option key={s.id} value={s.id}>
                    {s.name}
                    {s.exclusive ? ` · ${t("tags.exclusive")}` : ""}
                  </option>
                ))}
              </select>
            </label>
            {tagSet && !tagSet.exclusive && (
              <span style={{ color: "var(--pc-warn)", fontSize: 12 }}>
                {t("seqExport.notExclusive")}
              </span>
            )}
            <span className="muted" style={{ fontSize: 12 }}>
              {t("seqExport.namesHelp")}
            </span>
          </Section>
          <Section title={t("seqExport.format")}>
            <div style={{ display: "flex", flexWrap: "wrap", gap: 12 }}>
              <label style={{ display: "flex", flexDirection: "column", gap: 2 }}>
                {t("seqExport.formatLabel")}
                <select
                  className="field"
                  value={opts.format ?? "jpeg"}
                  onChange={(e) =>
                    set({ format: e.target.value as SequenceExportOptions["format"] })
                  }
                >
                  <option value="jpeg">JPEG</option>
                  <option value="original">{t("seqExport.originalCopy")}</option>
                </select>
              </label>
              <label style={{ display: "flex", flexDirection: "column", gap: 2 }}>
                {t("seqExport.quality")}
                <input
                  className="field"
                  type="number"
                  min={1}
                  max={100}
                  disabled={opts.format === "original"}
                  value={jpeg.quality ?? 97}
                  onChange={(e) =>
                    set({
                      jpeg: {
                        ...jpeg,
                        quality: Math.min(100, Math.max(1, Number(e.target.value) || 97)),
                      },
                    })
                  }
                  style={{ width: 80 }}
                />
              </label>
              <label style={{ display: "flex", flexDirection: "column", gap: 2 }}>
                {t("seqExport.chroma")}
                <select
                  className="field"
                  disabled={opts.format === "original"}
                  value={jpeg.chroma444 === false ? "420" : "444"}
                  onChange={(e) => set({ jpeg: { ...jpeg, chroma444: e.target.value === "444" } })}
                >
                  <option value="444">4:4:4</option>
                  <option value="420">4:2:0</option>
                </select>
              </label>
            </div>
            <label style={{ display: "flex", gap: 8 }}>
              <input
                type="checkbox"
                disabled={opts.format === "original"}
                checked={jpeg.verify !== false}
                onChange={(e) => set({ jpeg: { ...jpeg, verify: e.target.checked } })}
              />
              <span>{t("seqExport.verify")}</span>
            </label>
            <label style={{ display: "flex", gap: 8 }}>
              <input
                type="checkbox"
                disabled={opts.format === "original"}
                checked={opts.metadata}
                onChange={(e) => set({ metadata: e.target.checked })}
              />
              <span>{t("seqExport.metadata")}</span>
            </label>
            <span className="muted" style={{ fontSize: 12 }}>
              {t("seqExport.fidelity")}
            </span>
          </Section>
        </div>
        <div style={{ flex: "1 1 340px", display: "flex", flexDirection: "column", gap: 16 }}>
          <Section title={t("seqExport.structure")}>
            <label style={{ display: "flex", gap: 8, alignItems: "flex-start" }}>
              <input type="checkbox" checked disabled />
              <span>
                <span className="mono">sequenza/</span>
                <br />
                <span className="muted" style={{ fontSize: 12 }}>
                  {t("seqExport.outSequence")}
                </span>
              </span>
            </label>
            {check("perTag", "per-tag/<Tag>/", t("seqExport.outPerTag"), !opts.tagSet)}
            {check("manifest", "manifest.csv", t("seqExport.outManifest"))}
            {check("zip", "sequenza.zip", t("seqExport.outZip"))}
            {check("zipPerTag", "per-tag.zip", t("seqExport.outZip"), !opts.perTag)}
            <label style={{ display: "flex", flexDirection: "column", gap: 4 }}>
              {t("seqExport.destination")}
              <span style={{ display: "flex", gap: 8 }}>
                <input
                  className="field mono"
                  value={opts.destination}
                  aria-label={t("seqExport.destination")}
                  onChange={(e) => set({ destination: e.target.value })}
                  style={{ flex: 1 }}
                />
                <button className="btn" onClick={() => void browse()}>
                  {t("seqExport.browse")}
                </button>
              </span>
            </label>
          </Section>
          {pre && (
            <section
              role={pre.canStart ? "status" : "alert"}
              className="feature-card"
              style={{
                alignItems: "stretch",
                gap: 4,
                borderColor: pre.canStart ? "var(--pc-ok)" : "var(--pc-danger-border)",
              }}
            >
              <strong>
                {pre.canStart ? t("seqExport.preflightOk") : t("seqExport.preflightBlocked")}
              </strong>
              {pre.blocked && <span>{pre.blocked}</span>}
              <span>
                {t("seqExport.space", {
                  required: gb(pre.requiredBytes),
                  free: pre.freeBytes === null ? "—" : gb(pre.freeBytes),
                })}
              </span>
              {opts.perTag && (
                <span>
                  {pre.hardlinks ? t("seqExport.hardlinksYes") : t("seqExport.hardlinksNo")}
                </span>
              )}
            </section>
          )}
          {pre && opts.perTag && pre.untagged > 0 && (
            <section
              className="feature-card"
              style={{ alignItems: "stretch", color: "var(--pc-warn)" }}
            >
              {t("seqExport.untagged", { count: pre.untagged })}
            </section>
          )}
          <button
            className="btn btn-primary"
            style={{ minHeight: 44 }}
            disabled={!canStart}
            onClick={() => void start()}
          >
            {t("seqExport.start", { count: pre?.count ?? 0 })}
          </button>
          <span className="muted" style={{ fontSize: 12 }}>
            {t("seqExport.jobNote")}
          </span>
          {error && <p role="alert">{error}</p>}
          {started !== null && (
            <p role="status">
              {t("seqExport.started")}{" "}
              <a
                href="#"
                onClick={(e) => {
                  e.preventDefault();
                  useLayout.getState().setModule("jobs");
                }}
              >
                {t("seqExport.openJobs")}
              </a>
            </p>
          )}
        </div>
      </div>
    </div>
  );
}
