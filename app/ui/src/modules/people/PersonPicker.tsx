import { useQuery } from "@tanstack/react-query";
import { useMemo, useRef, useState, type ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { Button, Dialog, InlineAlert } from "../../components/ui";
import { SearchPicker, type PickerOption } from "../../components/ui/SearchPicker";
import { commands, isTauri } from "../../lib/ipc";
import { listAllGroups } from "./allGroups";
import { Cover } from "./Cover";
import "./people.css";

/** La destinazione scelta nel dialogo. */
export type PickerChoice =
  | { kind: "person"; person: number; name: string }
  | { kind: "identity"; uid: string; name: string }
  | { kind: "new"; name: string };

export type PickerConfirm = {
  title: string;
  body: string;
  /** Righe in più sotto il testo (es. chi verrà unito). */
  details?: string[];
  /** Avviso evidenziato prima del pulsante di conferma. */
  warning?: string;
};

const errorText = (e: unknown) =>
  (e as { detail?: string }).detail ?? (e instanceof Error ? e.message : String(e));

type Dest = PickerOption & { choice: PickerChoice };

export type PickerMode = "identities" | "assign" | "destination";

/**
 * Destinazioni possibili. "identities": tutte le identità dell'app (anche quelle già
 * selezionate: l'unione le ignora). "assign": le persone con un nome della libreria e le
 * identità non ancora collegate (la persona viene creata all'assegnazione). "destination"
 * ("Sposta in…", "Unisci a…"): prima le persone con nome, poi i gruppi senza nome, ciascuno con
 * il volto di copertina e il numero di foto, così che si distinguono a colpo d'occhio.
 */
function useDestinations(mode: PickerMode, exclude: number[]) {
  const { t } = useTranslation();
  return useQuery({
    queryKey: ["people-picker", mode, exclude],
    queryFn: async (): Promise<Dest[]> => {
      if (mode === "destination") {
        const [people, unnamed, view] = await Promise.all([
          listAllGroups("people", "name_asc"),
          listAllGroups("review", "photos_desc"),
          commands.listIdentities(null, null),
        ]);
        const skip = new Set(exclude);
        const persons = t("people.picker.persons");
        const groups = t("people.picker.unnamedGroups");
        // Persone in ordine alfabetico, con foto qui o no; poi i gruppi senza nome.
        const named: Dest[] = [
          ...people
            .filter((g) => g.name && !skip.has(g.id))
            .map((g) => ({
              value: `p:${g.id}`,
              label: g.name!,
              hint: t("people.photos", { count: g.photos }),
              icon: <Cover cover={g.cover} />,
              group: persons,
              choice: { kind: "person" as const, person: g.id, name: g.name! },
            })),
          ...view.identities
            .filter((i) => i.person == null)
            .map((i) => ({
              value: `i:${i.uid}`,
              label: i.name,
              hint: t("people.noPhotosInLibrary"),
              icon: <Cover cover={null} />,
              group: persons,
              choice: { kind: "identity" as const, uid: i.uid, name: i.name },
            })),
        ].sort((a, b) => a.label.localeCompare(b.label, undefined, { sensitivity: "base" }));
        return [
          ...named,
          ...unnamed
            .filter((g) => !g.name && !skip.has(g.id))
            .map((g) => ({
              value: `p:${g.id}`,
              label: t("people.unnamedPlain"),
              hint: t("people.photos", { count: g.photos }),
              icon: <Cover cover={g.cover} />,
              group: groups,
              choice: { kind: "person" as const, person: g.id, name: t("people.unnamedPlain") },
            })),
        ];
      }
      if (mode === "identities") {
        const view = await commands.listIdentities(null, null);
        return view.identities.map((i) => ({
          value: `i:${i.uid}`,
          label: i.name,
          hint:
            i.person != null && i.photos > 0
              ? t("people.bulk.photosCount", { count: i.photos })
              : (i.group ?? undefined),
          choice: { kind: "identity", uid: i.uid, name: i.name },
        }));
      }
      const [people, view] = await Promise.all([
        listAllGroups("people", "name_asc"),
        commands.listIdentities(null, null),
      ]);
      const named: Dest[] = people
        .filter((g) => g.name)
        .map((g) => ({
          value: `p:${g.id}`,
          label: g.name!,
          hint: t("people.bulk.photosCount", { count: g.photos }),
          choice: { kind: "person", person: g.id, name: g.name! },
        }));
      const unlinked: Dest[] = view.identities
        .filter((i) => i.person == null)
        .map((i) => ({
          value: `i:${i.uid}`,
          label: i.name,
          hint: t("people.bulk.noPhotosYet"),
          choice: { kind: "identity", uid: i.uid, name: i.name },
        }));
      return [...named, ...unlinked];
    },
    enabled: isTauri(),
    staleTime: 0,
    gcTime: 0,
  });
}

/**
 * Dialogo per scegliere una persona con la ricerca, riusato da "Sposta in…", "Unisci a…" e
 * dall'unione di più persone. Dopo la scelta mostra una conferma (con `describe`, che può
 * essere asincrona e fornire i conteggi) e solo con il pulsante esplicito esegue `run`.
 */
export function PersonPicker({
  mode,
  title,
  exclude = [],
  intro,
  allowCreate,
  describe,
  confirmLabel,
  danger,
  run,
  onClose,
}: {
  mode: PickerMode;
  title: string;
  /** Gruppi da non proporre (es. quello aperto). */
  exclude?: number[];
  intro: string;
  /** Mostra "Nuova persona «testo»" tra le scelte. */
  allowCreate?: boolean;
  /**
   * Conferma prima di eseguire (operazioni che non si annullano). Senza, la scelta esegue subito:
   * spostamenti e unioni tra gruppi si annullano dall'avviso (ui-foundations).
   */
  describe?: (choice: PickerChoice) => Promise<PickerConfirm> | PickerConfirm;
  confirmLabel?: (choice: PickerChoice) => string;
  danger?: boolean;
  run: (choice: PickerChoice) => Promise<unknown>;
  onClose: () => void;
}) {
  const { t } = useTranslation();
  const dest = useDestinations(mode, exclude);
  const [stage, setStage] = useState<
    { kind: "search" } | { kind: "confirm"; choice: PickerChoice; text: PickerConfirm }
  >({ kind: "search" });
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const locked = useRef(false);
  const options = dest.data;
  const byValue = useMemo(
    () => new Map((options ?? []).map((o) => [o.value, o.choice])),
    [options],
  );

  const pick = async (value: string, created: boolean) => {
    if (locked.current) return;
    const choice: PickerChoice | undefined = created
      ? { kind: "new", name: value }
      : byValue.get(value);
    if (!choice) return;
    locked.current = true;
    setBusy(true);
    setError("");
    try {
      if (!describe) {
        await run(choice);
        onClose();
        return;
      }
      setStage({ kind: "confirm", choice, text: await describe(choice) });
    } catch (e) {
      setError(errorText(e));
    } finally {
      locked.current = false;
      setBusy(false);
    }
  };

  const confirm = async () => {
    if (stage.kind !== "confirm" || locked.current) return;
    locked.current = true;
    setBusy(true);
    setError("");
    try {
      await run(stage.choice);
      onClose();
    } catch (e) {
      setError(errorText(e));
    } finally {
      locked.current = false;
      setBusy(false);
    }
  };

  let body: ReactNode;
  if (stage.kind === "confirm") {
    body = (
      <div className="people-stack people-stack--gap-sm">
        <p className="people-merge-body">{stage.text.body}</p>
        {stage.text.details?.map((d) => (
          <p key={d} className="muted people-merge-body">
            {d}
          </p>
        ))}
        {stage.text.warning && <InlineAlert tone="warn">{stage.text.warning}</InlineAlert>}
      </div>
    );
  } else if (dest.isError) {
    body = (
      <InlineAlert tone="danger" title={t("people.failed")}>
        {errorText(dest.error)}
      </InlineAlert>
    );
  } else if (!options) {
    body = (
      <p role="status" className="muted">
        {t("people.working")}
      </p>
    );
  } else {
    body = (
      <div className="people-stack people-stack--gap-sm">
        <p className="people-merge-body">{intro}</p>
        <SearchPicker
          label={t("people.bulk.searchLabel")}
          placeholder={t("people.bulk.searchPlaceholder")}
          options={options}
          onPick={pick}
          createLabel={allowCreate ? (name) => t("people.bulk.createNamed", { name }) : undefined}
          emptyLabel={t(allowCreate ? "people.bulk.emptyCreate" : "people.bulk.empty")}
        />
      </div>
    );
  }

  return (
    <Dialog
      open
      onOpenChange={(open) => !open && !busy && onClose()}
      title={stage.kind === "confirm" ? stage.text.title : title}
      footer={
        stage.kind === "confirm" ? (
          <>
            <Button
              variant="ghost"
              disabled={busy}
              onClick={() => {
                setError("");
                setStage({ kind: "search" });
              }}
            >
              {t("people.bulk.back")}
            </Button>
            <Button variant={danger ? "danger" : "primary"} disabled={busy} onClick={confirm}>
              {busy
                ? t("people.working")
                : (confirmLabel?.(stage.choice) ?? t("people.picker.confirm"))}
            </Button>
          </>
        ) : (
          <Button variant="ghost" disabled={busy} onClick={onClose}>
            {t("people.mergeCancel")}
          </Button>
        )
      }
    >
      {body}
      {error && (
        <InlineAlert tone="danger" title={t("people.failed")}>
          {error}
        </InlineAlert>
      )}
    </Dialog>
  );
}
