import { useQuery } from "@tanstack/react-query";
import { useMemo, useRef, useState, type ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { Button, Dialog, InlineAlert } from "../../components/ui";
import { SearchPicker, type PickerOption } from "../../components/ui/SearchPicker";
import { commands, isTauri } from "../../lib/ipc";
import { listAllGroups } from "./allGroups";
import "./people.css";

/** La destinazione scelta nel dialogo. */
export type PickerChoice =
  | { kind: "person"; person: number; name: string }
  | { kind: "identity"; uid: string; name: string }
  | { kind: "new"; name: string };

export type PickerConfirm = { title: string; body: string };

const errorText = (e: unknown) =>
  (e as { detail?: string }).detail ?? (e instanceof Error ? e.message : String(e));

type Dest = PickerOption & { choice: PickerChoice };

/**
 * Destinazioni possibili. "identities": tutte le identità dell'app (anche quelle già
 * selezionate: l'unione le ignora). "assign": le persone con un nome della libreria e le
 * identità non ancora collegate (la persona viene creata all'assegnazione).
 */
function useDestinations(mode: "identities" | "assign") {
  const { t } = useTranslation();
  return useQuery({
    queryKey: ["people-picker", mode],
    queryFn: async (): Promise<Dest[]> => {
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
 * Dialogo per scegliere una persona o un'identità con la ricerca, riusato da "Unisci in…",
 * "Assegna a…" e dai volti singoli. Dopo la scelta mostra una conferma (con `describe`, che può
 * essere asincrona e fornire i conteggi) e solo con il pulsante esplicito esegue `run`.
 */
export function PersonPicker({
  mode,
  title,
  intro,
  allowCreate,
  describe,
  confirmLabel,
  danger,
  run,
  onClose,
}: {
  mode: "identities" | "assign";
  title: string;
  intro: string;
  /** Mostra "Nuova persona «testo»" tra le scelte. */
  allowCreate?: boolean;
  describe: (choice: PickerChoice) => Promise<PickerConfirm> | PickerConfirm;
  confirmLabel: (choice: PickerChoice) => string;
  danger?: boolean;
  run: (choice: PickerChoice) => Promise<unknown>;
  onClose: () => void;
}) {
  const { t } = useTranslation();
  const dest = useDestinations(mode);
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
    body = <p className="people-merge-body">{stage.text.body}</p>;
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
              {busy ? t("people.working") : confirmLabel(stage.choice)}
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
