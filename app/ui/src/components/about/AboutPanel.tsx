// Informazioni su photo-cat (app-about): versione, build, sistema, autori, licenza e licenze di
// terze parti. Lo stesso pannello sta nella finestra Informazioni e in Impostazioni › Informazioni.
import { useQuery } from "@tanstack/react-query";
import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { commands, isTauri, type LicenseComponent } from "../../lib/ipc";
import { buildText, systemInfoLines } from "../../lib/systemInfo";
import { notifyError } from "../../store/toasts";
import { Button, InlineAlert, Tabs } from "../ui";
import { Icon } from "../icons";
import "./about.css";

type Tab = "info" | "licenses";

function InfoTab() {
  const { t } = useTranslation();
  const info = useQuery({
    queryKey: ["app-info"],
    queryFn: () => commands.appInfo(),
    enabled: isTauri(),
  });
  const [copyFailed, setCopyFailed] = useState(false);
  const data = info.data;
  if (!data) return null;
  const copy = async () => {
    setCopyFailed(false);
    try {
      // Senza il percorso dei dati: si incolla in una segnalazione (app-about).
      await navigator.clipboard.writeText(systemInfoLines(data, t).join("\n"));
    } catch {
      setCopyFailed(true);
    }
  };
  const license = data.license.replace(" OR ", ` ${t("about.or")} `);
  return (
    <div className="about-info">
      <p className="about-title">
        <Icon.Logo size={28} aria-hidden="true" />
        <span>
          <strong>photo-cat {data.version}</strong>
          <span className="muted">{t("about.tagline")}</span>
        </span>
      </p>
      <dl className="about-list">
        <dt>{t("about.build")}</dt>
        <dd>{buildText(data, t)}</dd>
        <dt>{t("about.system")}</dt>
        <dd>
          {data.os} {data.arch}
        </dd>
        <dt>{t("about.webview")}</dt>
        <dd>{data.webview ?? t("about.unknown")}</dd>
        <dt>{t("about.accelerator")}</dt>
        <dd>{data.accelerator}</dd>
        <dt>{t("about.dataDir")}</dt>
        <dd className="about-path">
          <span className="mono">{data.dataDir}</span>
          <Button
            size="sm"
            variant="ghost"
            onClick={() => commands.openDataDir().catch(notifyError)}
          >
            {t("about.open")}
          </Button>
        </dd>
        <dt>{t("about.authors")}</dt>
        <dd>{data.authors}</dd>
        {data.repository && (
          <>
            <dt>{t("about.source")}</dt>
            <dd className="mono">{data.repository}</dd>
          </>
        )}
        <dt>{t("about.license")}</dt>
        <dd>{license}</dd>
      </dl>
      <div className="about-actions">
        <Button icon={<Icon.Copy size={14} />} onClick={copy}>
          {t("about.copy")}
        </Button>
      </div>
      {copyFailed && <InlineAlert tone="danger" title={t("about.copyFailed")} />}
    </div>
  );
}

const KINDS = ["model", "native", "ui", "rust"] as const;

/** Percorso nell'installazione con il separatore del sistema (quello della cartella base). */
const joinPath = (root: string, relative: string) => {
  const sep = root.includes("\\") ? "\\" : "/";
  return `${root.replace(/[\\/]+$/, "")}${sep}${relative.split("/").join(sep)}`;
};

function LicensesTab() {
  const { t } = useTranslation();
  const [query, setQuery] = useState("");
  const [open, setOpen] = useState<string | null>(null);
  const licenses = useQuery({
    queryKey: ["third-party-licenses"],
    queryFn: () => commands.thirdPartyLicenses(),
    enabled: isTauri(),
    staleTime: Infinity,
  });
  const q = query.trim().toLowerCase();
  const groups = useMemo(() => {
    const list = licenses.data?.components ?? [];
    const shown = q
      ? list.filter((c) => `${c.name} ${c.version} ${c.license}`.toLowerCase().includes(q))
      : list;
    return KINDS.map((kind) => ({ kind, items: shown.filter((c) => c.kind === kind) })).filter(
      (g) => g.items.length > 0,
    );
  }, [licenses.data, q]);
  const key = (c: LicenseComponent) => `${c.kind}:${c.name}@${c.version}`;
  const total = groups.reduce((n, g) => n + g.items.length, 0);
  return (
    <div className="about-licenses">
      <input
        type="search"
        className="field"
        value={query}
        onChange={(e) => setQuery(e.target.value)}
        placeholder={t("about.licensesSearch")}
        aria-label={t("about.licensesSearch")}
      />
      {licenses.isError && <InlineAlert tone="danger" title={t("about.licensesFailed")} />}
      {licenses.data && (
        <p className="muted" role="status">
          {t("about.licensesCount", { count: total })}
        </p>
      )}
      <div className="about-licenses-list">
        {groups.map((g) => (
          <section key={g.kind} aria-label={t(`about.kinds.${g.kind}`)}>
            <h3 className="section-title">{t(`about.kinds.${g.kind}`)}</h3>
            <ul>
              {g.items.map((c) => {
                const id = key(c);
                const expanded = open === id;
                return (
                  <li key={id}>
                    <button
                      type="button"
                      className="about-license-row"
                      aria-expanded={expanded}
                      onClick={() => setOpen(expanded ? null : id)}
                    >
                      <span className="about-license-name">{c.name}</span>
                      <span className="muted">{c.version}</span>
                      <span className="about-license-id">{c.license}</span>
                    </button>
                    {expanded && (
                      <div className="about-license-text">
                        {c.sources && licenses.data && (
                          <p>
                            {t("about.sources")}{" "}
                            <span className="mono">
                              {joinPath(licenses.data.root ?? "", c.sources)}
                            </span>
                          </p>
                        )}
                        <pre className="mono">{licenses.data?.texts[c.text]}</pre>
                      </div>
                    )}
                  </li>
                );
              })}
            </ul>
          </section>
        ))}
      </div>
    </div>
  );
}

/** Pannello con le schede "Informazioni" e "Licenze di terze parti". */
export function AboutPanel({ initialTab = "info" }: { initialTab?: Tab }) {
  const { t } = useTranslation();
  const [tab, setTab] = useState<Tab>(initialTab);
  return (
    <Tabs
      label={t("about.title")}
      value={tab}
      onChange={setTab}
      tabs={[
        { value: "info", label: t("about.tabInfo"), content: <InfoTab /> },
        { value: "licenses", label: t("about.tabLicenses"), content: <LicensesTab /> },
      ]}
    />
  );
}
