import { useState } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { commands, isTauri, type FaceChoice } from "../../lib/ipc";
import { FaceCrop } from "./FaceCrop";
import { Icon } from "../../components/icons";
import { Button, InlineAlert } from "../../components/ui";
import "./people.css";

const errorText = (e: unknown) => (e as { detail?: string }).detail ?? String(e);

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

/**
 * "Volti di riferimento" di una persona: aiutano a riconoscerla in altre librerie e in foto di anni
 * diversi; si aggiungono da un file o dai volti della libreria (identity-registry).
 */
export function ReferenceFaces({
  uid,
  name,
  person,
}: {
  uid: string;
  name: string;
  /** Gruppo della persona nella libreria aperta, se c'è. */
  person: number | null;
}) {
  const { t } = useTranslation();
  const qc = useQueryClient();
  const [choices, setChoices] = useState<{ path: string; faces: FaceChoice[] } | null>(null);
  const [fromLibrary, setFromLibrary] = useState(false);
  const [error, setError] = useState("");
  const [notice, setNotice] = useState("");
  const faces = useQuery({
    queryKey: ["identity-faces", uid],
    queryFn: () => commands.identityFaces(uid),
    enabled: isTauri(),
  });
  const libraryFaces = useQuery({
    queryKey: ["faces", person],
    queryFn: () => commands.personFaces(person!, 500),
    enabled: isTauri() && fromLibrary && person != null,
  });
  const run = async (fn: () => Promise<unknown>) => {
    setError("");
    try {
      await fn();
      await Promise.all([
        qc.invalidateQueries({ queryKey: ["identity-faces"] }),
        qc.invalidateQueries({ queryKey: ["identities"] }),
      ]);
    } catch (e) {
      setError(errorText(e));
    }
  };
  const addFile = (path: string, choice: number | null) =>
    run(async () => {
      const out = await commands.addIdentityReferenceFile(uid, path, choice);
      if (out.outcome === "choose") setChoices({ path, faces: out.faces });
      else {
        setChoices(null);
        if (out.outcome === "noFace") setNotice(t("identities.noFace"));
      }
    });
  const pickFile = async () => {
    const { open } = await import("@tauri-apps/plugin-dialog");
    const path = await open({
      multiple: false,
      filters: [
        {
          name: t("identities.imagesFilter"),
          extensions: ["jpg", "jpeg", "png", "heic", "heif", "tif", "tiff", "webp"],
        },
      ],
    });
    if (typeof path === "string") await addFile(path, null);
  };
  return (
    <div className="people-stack people-stack--gap-md people-pad">
      <p className="muted people-muted-reset">{t("identities.referencesHint", { name })}</p>
      <div className="people-wrap">
        {faces.data?.map((f) => (
          <RefFace
            key={f.id}
            id={f.id}
            onRemove={() => void run(() => commands.removeIdentityFace(f.id))}
          />
        ))}
      </div>
      <div className="people-wrap people-wrap--gap-sm">
        <Button icon={<Icon.Plus size={14} />} onClick={() => void pickFile()}>
          {t("identities.addFromFile")}
        </Button>
        {person != null && (
          <Button onClick={() => setFromLibrary((v) => !v)} aria-pressed={fromLibrary}>
            {t("identities.addFromLibrary")}
          </Button>
        )}
      </div>
      {notice && <p role="status">{notice}</p>}
      {error && (
        <InlineAlert tone="danger" title={t("people.failed")}>
          {error}
        </InlineAlert>
      )}
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
                    await commands.addIdentityReferenceFace(uid, f.id);
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
    </div>
  );
}
