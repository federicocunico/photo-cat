import { useEffect, useRef, useState } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import {
  commands,
  isTauri,
  type FaceChoice,
  type IdentityInput,
  type IdentityRow,
} from "../../lib/ipc";
import { fmtDate } from "../../lib/format";
import { FaceCrop } from "./FaceCrop";
import { OpenInPeople } from "./OpenInPeople";
import { Icon } from "../../components/icons";
import { Button, ConfirmDialog, InlineAlert } from "../../components/ui";
import "./people.css";

const errorText = (e: unknown) => (e as { detail?: string }).detail ?? String(e);
const KEY = ["identities"] as const;

const initials = (name: string) =>
  name
    .split(/\s+/)
    .filter(Boolean)
    .slice(0, 2)
    .map((w) => w[0]?.toUpperCase())
    .join("");

function Avatar({ name, size = 36 }: { name: string; size?: number }) {
  return (
    <span
      aria-hidden="true"
      className="people-avatar"
      style={{
        width: size,
        height: size,
        fontSize: size / 3,
      }}
    >
      {initials(name)}
    </span>
  );
}

function RefFace({ id, onRemove }: { id: number; onRemove: () => void }) {
  const { t } = useTranslation();
  const img = useQuery({
    queryKey: ["identity-face", id],
    queryFn: () => commands.identityFaceImage(id),
    enabled: isTauri(),
    staleTime: Infinity,
  });
  return (
    <span className="people-ref">
      {img.data ? (
        <img src={img.data} alt="" width={64} height={64} className="people-ref-img" />
      ) : (
        <span className="people-ref-ph" />
      )}
      <button
        type="button"
        className="btn icon-btn people-ref-remove"
        aria-label={t("identities.removeFace")}
        onClick={onRemove}
      >
        <Icon.Close size={14} />
      </button>
    </span>
  );
}

type Draft = { name: string; shortName: string; group: string; aliases: string; threshold: number };

const toDraft = (r: IdentityRow | null, def: number): Draft => ({
  name: r?.name ?? "",
  shortName: r?.shortName ?? "",
  group: r?.group ?? "",
  aliases: r?.aliases.join(", ") ?? "",
  threshold: r?.threshold ?? def,
});

const toInput = (d: Draft, def: number): IdentityInput => ({
  name: d.name.trim(),
  shortName: d.shortName.trim() || null,
  group: d.group.trim() || null,
  threshold: Math.abs(d.threshold - def) < 1e-6 ? null : d.threshold,
  aliases: d.aliases
    .split(",")
    .map((a) => a.trim())
    .filter(Boolean),
});

/** Editor di un'identità: nome, alias, gruppo, soglia, riferimenti, unione ed eliminazione. */
export function IdentityEditor({
  row,
  creating,
  all,
  groups,
  defaultThreshold,
  onSaved,
  onDeleted,
  compact = false,
}: {
  row: IdentityRow | null;
  creating: boolean;
  all: IdentityRow[];
  groups: string[];
  defaultThreshold: number;
  onSaved: (uid: string) => void;
  onDeleted: () => void;
  /** Layout più stretto per la colonna scheda del modulo Persone. */
  compact?: boolean;
}) {
  const { t } = useTranslation();
  const qc = useQueryClient();
  const [draft, setDraft] = useState<Draft>(toDraft(row, defaultThreshold));
  const [error, setError] = useState("");
  const [choices, setChoices] = useState<{ path: string; faces: FaceChoice[] } | null>(null);
  const [fromLibrary, setFromLibrary] = useState(false);
  const [mergeInto, setMergeInto] = useState<string | null>(null);
  const [confirmDelete, setConfirmDelete] = useState(false);
  const [confirmMerge, setConfirmMerge] = useState(false);
  const [notice, setNotice] = useState("");
  useEffect(() => {
    setDraft(toDraft(row, defaultThreshold));
    setError("");
    setChoices(null);
    setFromLibrary(false);
    setMergeInto(null);
    setConfirmDelete(false);
    setConfirmMerge(false);
    setNotice("");
  }, [row?.uid, creating, defaultThreshold]); // eslint-disable-line react-hooks/exhaustive-deps
  const faces = useQuery({
    queryKey: ["identity-faces", row?.uid],
    queryFn: () => commands.identityFaces(row!.uid),
    enabled: isTauri() && !!row,
  });
  const libraryFaces = useQuery({
    queryKey: ["faces", row?.person],
    queryFn: () => commands.personFaces(row!.person, 500),
    enabled: isTauri() && fromLibrary && row?.person != null,
  });
  const refresh = () =>
    Promise.all([
      qc.invalidateQueries({ queryKey: KEY }),
      qc.invalidateQueries({ queryKey: ["identity-faces"] }),
      qc.invalidateQueries({ queryKey: ["people"] }),
      qc.invalidateQueries({ queryKey: ["people-groups"] }),
      qc.invalidateQueries({ queryKey: ["people-sections"] }),
    ]);
  // Blocco sincrono: un doppio clic o un doppio invio non manda due volte la stessa azione.
  const locked = useRef(false);
  const [busy, setBusy] = useState(false);
  const run = async (fn: () => Promise<unknown>) => {
    if (locked.current) return;
    locked.current = true;
    setBusy(true);
    setError("");
    try {
      await fn();
      await refresh();
    } catch (e) {
      setError(errorText(e));
    } finally {
      locked.current = false;
      setBusy(false);
    }
  };
  const save = () =>
    run(async () => {
      const uid = await commands.saveIdentity(
        creating ? null : row!.uid,
        toInput(draft, defaultThreshold),
      );
      onSaved(uid);
    });
  const addFile = async (path: string, choice: number | null) => {
    if (!row) return;
    await run(async () => {
      const out = await commands.addIdentityReferenceFile(row.uid, path, choice);
      if (out.outcome === "choose") setChoices({ path, faces: out.faces });
      else {
        setChoices(null);
        if (out.outcome === "noFace") setNotice(t("identities.noFace"));
      }
    });
  };
  const pickFile = async () => {
    const { open } = await import("@tauri-apps/plugin-dialog");
    const path = await open({
      multiple: false,
      filters: [
        {
          name: "Immagini",
          extensions: ["jpg", "jpeg", "png", "heic", "heif", "tif", "tiff", "webp"],
        },
      ],
    });
    if (typeof path === "string") await addFile(path, null);
  };
  const field = (key: keyof Draft, label: string, extra: Record<string, unknown> = {}) => (
    <label className="people-field-col">
      <span className="people-field-label">{label}</span>
      <input
        className="field people-field-input"
        value={String(draft[key])}
        onChange={(e) => setDraft({ ...draft, [key]: e.target.value })}
        {...extra}
      />
    </label>
  );
  const mergeName = all.find((x) => x.uid === mergeInto)?.name ?? "";
  if (!row && !creating) return <p className="muted">{t("identities.none")}</p>;
  const title = creating ? t("identities.new") : row!.name;
  return (
    <div className="people-stack" style={{ gap: compact ? 12 : 14 }}>
      <div className="people-inline people-inline--gap-lg">
        <Avatar name={draft.name || "?"} size={compact ? 40 : 48} />
        <div>
          <div className="muted people-muted-sm">
            {row ? t("identities.createdAt", { date: fmtDate(new Date(row.createdAt)) }) : ""}
          </div>
          <strong style={{ fontSize: compact ? 16 : 20 }}>{title}</strong>
        </div>
      </div>
      <form
        onSubmit={(e) => {
          e.preventDefault();
          void save();
        }}
        className="people-stack people-stack--gap-md"
      >
        {field("name", t("identities.name"), { required: true })}
        {field("shortName", t("identities.shortName"), {
          placeholder: draft.name.trim().split(/\s+/)[0] ?? "",
        })}
        {field("group", t("identities.group"), { list: "pc-identity-groups" })}
        <datalist id="pc-identity-groups">
          {groups.map((g) => (
            <option key={g} value={g} />
          ))}
        </datalist>
        {field("aliases", t("identities.aliases"), { placeholder: t("identities.aliasesHint") })}
        <label className="people-field-col">
          <span className="people-field-head">
            <span className="people-field-label">{t("identities.threshold")}</span>
            <span className="mono">{draft.threshold.toFixed(2)}</span>
          </span>
          <input
            type="range"
            min={0}
            max={100}
            value={Math.round(draft.threshold * 100)}
            aria-label={t("identities.threshold")}
            onChange={(e) => setDraft({ ...draft, threshold: Number(e.target.value) / 100 })}
          />
        </label>
        <button
          className="btn btn-primary"
          type="submit"
          disabled={!draft.name.trim()}
          aria-busy={busy || undefined}
        >
          {creating ? t("identities.create") : t("identities.save")}
        </button>
      </form>
      {error && (
        <InlineAlert tone="danger" title={t("people.failed")}>
          {error}
        </InlineAlert>
      )}
      {notice && <p role="status">{notice}</p>}
      {row && (
        <>
          <div className="people-inline people-inline--wrap">
            <h2 className="people-section-title">
              {t("identities.references", { count: faces.data?.length ?? row.faces })}
            </h2>
            <button className="btn people-ml-auto" onClick={() => void pickFile()}>
              {t("identities.addFromFile")}
            </button>
            {row.person != null && (
              <button className="btn" onClick={() => setFromLibrary((v) => !v)}>
                {t("identities.addFromLibrary")}
              </button>
            )}
          </div>
          <div className="people-wrap">
            {faces.data?.map((f) => (
              <RefFace
                key={f.id}
                id={f.id}
                onRemove={() => void run(() => commands.removeIdentityFace(f.id))}
              />
            ))}
          </div>
          <span className="muted people-muted-sm">{t("identities.referencesHint")}</span>
          {choices && (
            <div role="group" aria-label={t("identities.chooseFace")} className="feature-card">
              <span>{t("identities.chooseFace")}</span>
              <div className="people-wrap">
                {choices.faces.map((f) => (
                  <button
                    key={f.index}
                    type="button"
                    className="btn people-face-pick"
                    aria-label={t("identities.faceN", { n: f.index + 1 })}
                    onClick={() => void addFile(choices.path, f.index)}
                  >
                    <img src={f.crop} alt="" width={72} height={72} />
                  </button>
                ))}
              </div>
            </div>
          )}
          {fromLibrary && (
            <div className="feature-card people-card-stretch">
              <span>{t("identities.chooseLibraryFace")}</span>
              <div className="people-wrap">
                {libraryFaces.data?.slice(0, 24).map((f, i) => (
                  <button
                    key={f.id}
                    type="button"
                    className="btn people-face-pick"
                    aria-label={t("identities.libraryFaceN", { n: i + 1 })}
                    onClick={() =>
                      void run(async () => {
                        await commands.addIdentityReferenceFace(row.uid, f.id);
                        setFromLibrary(false);
                      })
                    }
                  >
                    <FaceCrop face={f} />
                  </button>
                ))}
              </div>
            </div>
          )}
          <div className="people-soft">
            {row.person != null && row.photos > 0 ? (
              <>
                {t("identities.linked", { count: row.photos })}{" "}
                <OpenInPeople
                  target={{ person: row.person }}
                  className="btn btn-ghost people-open-inline"
                />
              </>
            ) : (
              t("people.noPhotosInLibrary")
            )}
          </div>
          <div className="people-stack people-stack--gap">
            {mergeInto === null ? (
              <button className="btn" onClick={() => setMergeInto("")}>
                {t("identities.mergeWith")}
              </button>
            ) : (
              <div className="feature-card people-card-stretch">
                <select
                  className="field"
                  aria-label={t("identities.mergeTarget")}
                  value={mergeInto}
                  onChange={(e) => setMergeInto(e.target.value)}
                >
                  <option value="">{t("identities.mergeTarget")}</option>
                  {all
                    .filter((x) => x.uid !== row.uid)
                    .map((x) => (
                      <option key={x.uid} value={x.uid}>
                        {x.name}
                      </option>
                    ))}
                </select>
                {mergeInto && (
                  <p className="people-muted-reset">
                    {t("identities.mergeConfirm", {
                      from: row.name,
                      into: all.find((x) => x.uid === mergeInto)?.name ?? "",
                    })}
                  </p>
                )}
                <div className="feature-actions people-actions-flush">
                  <Button
                    variant="primary"
                    disabled={!mergeInto}
                    onClick={() => setConfirmMerge(true)}
                  >
                    {t("identities.merge")}
                  </Button>
                  <Button onClick={() => setMergeInto(null)}>{t("identities.cancel")}</Button>
                </div>
              </div>
            )}
            <Button variant="danger" onClick={() => setConfirmDelete(true)}>
              {t("identities.delete")}
            </Button>
            <ConfirmDialog
              open={confirmMerge && !!mergeInto}
              onOpenChange={setConfirmMerge}
              title={t("identities.mergeTitle", { from: row.name, into: mergeName })}
              message={t("identities.mergeConfirm", { from: row.name, into: mergeName })}
              confirmLabel={t("identities.mergeButton", { into: mergeName })}
              danger={false}
              onConfirm={() =>
                run(async () => {
                  await commands.mergeIdentities(row.uid, mergeInto!);
                  onSaved(mergeInto!);
                })
              }
            />
            <ConfirmDialog
              open={confirmDelete}
              onOpenChange={setConfirmDelete}
              title={t("identities.deleteTitle", { name: row.name })}
              message={t("identities.deleteConfirm", { name: row.name })}
              confirmLabel={t("identities.deleteButton")}
              onConfirm={() =>
                run(async () => {
                  await commands.deleteIdentity(row.uid);
                  onDeleted();
                })
              }
            />
          </div>
        </>
      )}
    </div>
  );
}
