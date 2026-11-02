import { QueryView } from "../../components/feedback/QueryView";
import { BusyButton } from "../../components/feedback/BusyButton";
import { useState } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { commands, isTauri, type Correction, type FaceView } from "../../lib/ipc";
import { faceUrl, thumbUrl } from "../library/thumbUrl";
import { WhoIs } from "../identities/WhoIs";

export function FaceCrop({ face }: { face: FaceView }) {
  const [x, y, w, h] = face.bbox.map((v) => v ?? 0);
  const [failed, setFailed] = useState(false);
  const src = faceUrl(face.id);
  return (
    <span
      style={{
        display: "block",
        position: "relative",
        overflow: "hidden",
        width: 112,
        height: 112,
        backgroundColor: "var(--pc-panel)",
        // Segnaposto immediato dalla miniatura; il ritaglio ad alta risoluzione lo copre appena pronto.
        backgroundImage: face.thumb ? `url("${thumbUrl(face.thumb)}")` : undefined,
        backgroundSize: `${100 / Math.max(w, 0.01)}% ${100 / Math.max(h, 0.01)}%`,
        backgroundPosition: `${(100 * x) / Math.max(1 - w, 0.01)}% ${(100 * y) / Math.max(1 - h, 0.01)}%`,
        borderRadius: 6,
      }}
    >
      {src && !failed && (
        <img
          src={src}
          alt=""
          loading="lazy"
          decoding="async"
          onError={() => setFailed(true)}
          style={{
            position: "absolute",
            inset: 0,
            width: "100%",
            height: "100%",
            objectFit: "cover",
          }}
        />
      )}
    </span>
  );
}
export function PeoplePage() {
  const { t } = useTranslation();
  const qc = useQueryClient();
  const [person, setPerson] = useState<number | null>(null);
  const [detail, setDetail] = useState(false);
  const [selected, setSelected] = useState<number[]>([]);
  const [target, setTarget] = useState("");
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);
  const people = useQuery({
    queryKey: ["people"],
    queryFn: commands.listPeople,
    enabled: isTauri(),
    refetchInterval: 5000,
  });
  const faces = useQuery({
    queryKey: ["faces", person],
    queryFn: () => commands.personFaces(person),
    enabled: isTauri() && detail,
    refetchInterval: 5000,
  });
  const active = people.data?.find((p) => p.id === person);
  const correct = async (c: Correction) => {
    setBusy(true);
    setError("");
    try {
      await commands.correctPerson(c);
      setSelected([]);
      await qc.invalidateQueries({ queryKey: ["people"] });
      await qc.invalidateQueries({ queryKey: ["faces"] });
    } catch (e) {
      setError((e as { detail?: string }).detail ?? String(e));
    } finally {
      setBusy(false);
    }
  };
  const open = (id: number | null) => {
    setPerson(id);
    setDetail(true);
    setSelected([]);
  };
  return (
    <QueryView query={people} enabled={isTauri()} isEmpty={() => false}>
      {() => (
        <QueryView query={faces} enabled={isTauri() && detail} isEmpty={() => false}>
          {() => (
            <section style={{ overflow: "auto", padding: 24, flex: 1 }}>
              <h1>{t("people.title")}</h1>
              {error && <p role="alert">{error}</p>}
              {!detail ? (
                <>
                  <BusyButton className="btn" onClick={() => open(null)}>
                    {t("people.all")}
                  </BusyButton>
                  {!people.data?.length && <p className="muted">{t("people.empty")}</p>}
                  <div className="feature-grid">
                    {people.data?.map((p) => (
                      <BusyButton className="feature-card" key={p.id} onClick={() => open(p.id)}>
                        {p.cover && <FaceCrop face={p.cover} />}
                        <strong>{p.name ?? `${t("people.unnamed")} ${p.id}`}</strong>
                        <span>{t("people.photos", { count: p.photos })}</span>
                      </BusyButton>
                    ))}
                  </div>
                </>
              ) : (
                <>
                  <BusyButton className="btn" onClick={() => setDetail(false)}>
                    {t("people.back")}
                  </BusyButton>
                  <h2>{active?.name ?? t("people.all")}</h2>
                  <div className="feature-actions">
                    {person !== null && (
                      <WhoIs
                        person={person}
                        linked={active?.identityUid ? (active.name ?? null) : null}
                        onLinked={(p) => setPerson(p)}
                      />
                    )}
                    <select
                      className="field"
                      aria-label={t("people.target")}
                      value={target}
                      onChange={(e) => setTarget(e.target.value)}
                    >
                      <option value="">{t("people.target")}</option>
                      {people.data
                        ?.filter((p) => p.id !== person)
                        .map((p) => (
                          <option value={p.id} key={p.id}>
                            {p.name ?? `${t("people.unnamed")} ${p.id}`}
                          </option>
                        ))}
                    </select>
                    {person !== null && (
                      <BusyButton
                        className="btn"
                        disabled={busy || !target}
                        onClick={async () =>
                          await correct({ action: "merge", from: person, to: Number(target) }).then(
                            () => open(Number(target)),
                          )
                        }
                      >
                        {t("people.merge")}
                      </BusyButton>
                    )}
                    <BusyButton
                      className="btn"
                      disabled={busy || !target || !selected.length}
                      onClick={async () =>
                        await correct({ action: "assign", person: Number(target), faces: selected })
                      }
                    >
                      {t("people.assign")}
                    </BusyButton>
                    {person !== null && (
                      <BusyButton
                        className="btn"
                        disabled={busy || !selected.length}
                        onClick={async () =>
                          await correct({ action: "separate", person, faces: selected })
                        }
                      >
                        {t("people.separate")}
                      </BusyButton>
                    )}
                    <span>{t("people.selected", { count: selected.length })}</span>
                  </div>
                  <div className="feature-grid">
                    {faces.data?.map((f) => (
                      <article className="feature-card" key={f.id}>
                        <label>
                          <input
                            type="checkbox"
                            aria-label={`#${f.id}`}
                            checked={selected.includes(f.id)}
                            onChange={(e) =>
                              setSelected((s) =>
                                e.target.checked ? [...s, f.id] : s.filter((id) => id !== f.id),
                              )
                            }
                          />
                          <FaceCrop face={f} />
                        </label>
                        {f.personId !== null && (
                          <BusyButton
                            className="btn"
                            disabled={busy}
                            onClick={async () =>
                              await correct({ action: "reject", person: f.personId!, face: f.id })
                            }
                          >
                            {t("people.reject")}
                          </BusyButton>
                        )}
                        <BusyButton
                          className="btn"
                          disabled={busy}
                          onClick={async () => {
                            if (window.confirm(t("people.removeConfirm")))
                              await correct({ action: "remove", face: f.id });
                          }}
                        >
                          {t("people.remove")}
                        </BusyButton>
                      </article>
                    ))}
                  </div>
                </>
              )}
            </section>
          )}
        </QueryView>
      )}
    </QueryView>
  );
}
