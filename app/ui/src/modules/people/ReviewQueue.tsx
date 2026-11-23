import { useEffect, useRef, useState, type KeyboardEvent } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import {
  commands,
  isTauri,
  type Answer,
  type AnswerConflict,
  type AnswerOutcome,
  type QuestionGroup,
  type ReviewItem,
  type ReviewQuestion,
} from "../../lib/ipc";
import { Button, InlineAlert } from "../../components/ui";
import { faceUrl } from "../library/thumbUrl";
import { Cover } from "./Cover";
import { notifyPeopleUndo, refreshPeopleFast } from "./peopleUndo";
import "./people.css";

export const QUEUE_KEY = ["review-queue"] as const;
/** Elementi chiesti al backend per volta: il resto arriva man mano che si risponde (D6). */
const QUEUE_LIMIT = 50;

const errorText = (e: unknown) => (e as { detail?: string }).detail ?? String(e);
const coverOf = (g: QuestionGroup) => (g.cover != null ? { id: g.cover, thumb: null } : null);

/** Probabilità stimata in parole: non è una somiglianza, serve solo a dare un'idea. */
function likelihoodKey(p: number | null) {
  if (p == null) return null;
  return p >= 0.8
    ? "people.queue.likely.high"
    : p >= 0.5
      ? "people.queue.likely.medium"
      : "people.queue.likely.low";
}

/** I volti di un gruppo, per decidere guardandoli e non solo dalla copertina. */
function GroupFaces({ group, max = 12 }: { group: QuestionGroup; max?: number }) {
  const { t } = useTranslation();
  const faces = useQuery({
    queryKey: ["queue-faces", group.id, max],
    queryFn: () =>
      commands.groupFaces({ kind: "group", group: group.id }, "quality_desc", null, max),
    enabled: isTauri(),
    staleTime: 60_000,
  });
  const items = faces.data?.items ?? [];
  return (
    <div
      className="people-queue-faces"
      role="group"
      aria-label={t("people.queue.facesOf", { name: group.name ?? t("people.unnamedPlain") })}
    >
      {items.map((f) => (
        <img
          key={f.id}
          src={faceUrl(f.id) ?? undefined}
          alt=""
          loading="lazy"
          decoding="async"
          className="people-queue-face"
        />
      ))}
    </div>
  );
}

/** Nome di un gruppo della domanda: il nome della persona o "Persona senza nome". */
function useName() {
  const { t } = useTranslation();
  return (g: QuestionGroup) => g.name ?? t("people.unnamedPlain");
}

/** Una domanda: "È Gioia?", "È la stessa persona?" o "È Anna o Marta?". */
function SingleQuestion({
  question,
  busy,
  onAnswer,
}: {
  question: ReviewQuestion;
  busy: boolean;
  onAnswer: (answer: Answer) => void;
}) {
  const { t } = useTranslation();
  const name = useName();
  const { kind, group, other, second } = question;
  const title =
    kind === "group_person"
      ? t("people.queue.isName", { name: name(other) })
      : kind === "group_choice" && second
        ? t("people.queue.isAorB", { a: name(other), b: name(second) })
        : t("people.queue.same");
  const likely = likelihoodKey(question.probability);
  return (
    <section className="people-queue-item" aria-labelledby={`queue-q-${question.id}`}>
      <h3 id={`queue-q-${question.id}`} className="people-question-title">
        {title}
      </h3>
      {likely && <p className="muted people-muted-sm">{t(likely)}</p>}
      <div className="people-queue-sides">
        <div className="people-queue-side">
          <div className="people-inline">
            <Cover cover={coverOf(group)} size="md" />
            <span>
              <strong>{name(group)}</strong>
              <span className="muted people-muted-sm">
                {" "}
                · {t("people.photos", { count: group.photos })}
              </span>
            </span>
          </div>
          <GroupFaces group={group} />
        </div>
        {[other, ...(kind === "group_choice" && second ? [second] : [])].map((g) => (
          <div key={g.id} className="people-queue-side">
            <div className="people-inline">
              <Cover cover={coverOf(g)} size="md" />
              <span>
                <strong>{name(g)}</strong>
                <span className="muted people-muted-sm">
                  {" "}
                  · {t("people.photos", { count: g.photos })}
                </span>
              </span>
            </div>
            <GroupFaces group={g} max={6} />
          </div>
        ))}
      </div>
      <p className="muted people-muted-sm">
        {t(kind === "group_choice" ? "people.queue.keysChoice" : "people.queue.keys")}
      </p>
      <div className="people-question-actions">
        {kind === "group_choice" && second ? (
          <>
            <Button
              variant="primary"
              disabled={busy}
              onClick={() => onAnswer({ kind: "choose", person: other.id })}
            >
              {name(other)}
            </Button>
            <Button
              variant="primary"
              disabled={busy}
              onClick={() => onAnswer({ kind: "choose", person: second.id })}
            >
              {name(second)}
            </Button>
            <Button disabled={busy} onClick={() => onAnswer({ kind: "neither" })}>
              {t("people.queue.neither")}
            </Button>
          </>
        ) : (
          <>
            <Button variant="primary" disabled={busy} onClick={() => onAnswer({ kind: "yes" })}>
              {kind === "group_person"
                ? t("people.question.yesName", { name: name(other) })
                : t("people.question.yes")}
            </Button>
            <Button disabled={busy} onClick={() => onAnswer({ kind: "no" })}>
              {t("people.question.no")}
            </Button>
          </>
        )}
      </div>
    </section>
  );
}

/** "12 gruppi probabilmente di Gioia": tutti i volti, caselle e "Conferma tutti". */
function BatchQuestion({
  person,
  questions,
  busy,
  unchecked,
  onToggle,
  onConfirm,
}: {
  person: QuestionGroup;
  questions: ReviewQuestion[];
  busy: boolean;
  unchecked: ReadonlySet<number>;
  onToggle: (question: number) => void;
  onConfirm: () => void;
}) {
  const { t } = useTranslation();
  const name = useName();
  const kept = questions.filter((q) => !unchecked.has(q.id)).length;
  return (
    <section className="people-queue-item" aria-labelledby={`queue-b-${person.id}`}>
      <div className="people-inline">
        <Cover cover={coverOf(person)} size="md" />
        <h3 id={`queue-b-${person.id}`} className="people-question-title">
          {t("people.queue.batchTitle", { count: questions.length, name: name(person) })}
        </h3>
      </div>
      <p className="muted people-muted-sm">{t("people.queue.batchHint", { name: name(person) })}</p>
      <ul className="people-queue-batch">
        {questions.map((q) => {
          const checked = !unchecked.has(q.id);
          return (
            <li key={q.id} className="people-queue-batch-item" data-checked={checked || undefined}>
              <label className="people-inline">
                <input
                  type="checkbox"
                  checked={checked}
                  disabled={busy}
                  onChange={() => onToggle(q.id)}
                />
                <Cover cover={coverOf(q.group)} />
                <span>
                  {t("people.queue.groupOf", { count: q.group.faces })}
                  <span className="muted people-muted-sm">
                    {" "}
                    · {t("people.photos", { count: q.group.photos })}
                  </span>
                </span>
              </label>
              <GroupFaces group={q.group} max={8} />
            </li>
          );
        })}
      </ul>
      <p className="muted people-muted-sm">{t("people.queue.keysBatch")}</p>
      <div className="people-question-actions">
        <Button variant="primary" disabled={busy || questions.length === 0} onClick={onConfirm}>
          {t("people.queue.confirmAll", { count: kept })}
        </Button>
      </div>
    </section>
  );
}

/**
 * Coda "Da confermare" (face-recognition, D6): una domanda alla volta, o un blocco di gruppi
 * verso la stessa persona. Ogni risposta è un'operazione annullabile; i conflitti con un vincolo
 * precedente si mostrano senza unire nulla. Da tastiera: S/N, 1/2, Invio, → per saltare.
 */
export function ReviewQueuePanel() {
  const { t } = useTranslation();
  const qc = useQueryClient();
  const name = useName();
  const queue = useQuery({
    queryKey: QUEUE_KEY,
    queryFn: () => commands.reviewQueue(QUEUE_LIMIT),
    enabled: isTauri(),
  });
  const [skipped, setSkipped] = useState(0);
  const [unchecked, setUnchecked] = useState<ReadonlySet<number>>(new Set());
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const [conflict, setConflict] = useState<{ conflict: AnswerConflict; retry: () => void } | null>(
    null,
  );
  const panelRef = useRef<HTMLDivElement>(null);

  const items = queue.data?.items ?? [];
  const index = items.length ? skipped % items.length : 0;
  const item: ReviewItem | undefined = items[index];
  const itemKey = item
    ? item.kind === "single"
      ? `q${item.question.id}`
      : `b${item.person.id}`
    : "";

  // Nuovo elemento: caselle di nuovo tutte selezionate, nessun conflitto in sospeso.
  useEffect(() => {
    setUnchecked(new Set());
    setConflict(null);
    setError("");
  }, [itemKey]);
  useEffect(() => {
    panelRef.current?.focus();
  }, []);

  /** Esegue una risposta; un conflitto si mostra e si risolve con `override`. */
  const run = async (
    call: (override: boolean) => Promise<AnswerOutcome>,
    done: (o: AnswerOutcome) => string,
    override = false,
  ) => {
    setBusy(true);
    setError("");
    try {
      const out = await call(override);
      if (out.conflict) {
        const c = out.conflict;
        setConflict({ conflict: c, retry: () => void run(call, done, true) });
        return;
      }
      setConflict(null);
      notifyPeopleUndo(qc, done(out));
      await Promise.all([qc.invalidateQueries({ queryKey: QUEUE_KEY }), refreshPeopleFast(qc)]);
    } catch (e) {
      setError(errorText(e));
    } finally {
      setBusy(false);
    }
  };

  const answer = (q: ReviewQuestion, a: Answer) => {
    const target =
      a.kind === "choose" ? (a.person === q.other.id ? q.other : (q.second ?? q.other)) : q.other;
    void run(
      (override) => commands.answerQuestion(q.id, a, override),
      (o) =>
        o.mergedGroups > 0
          ? t("people.queue.mergedDone", { count: o.mergedGroups, name: name(target) })
          : t("people.queue.constrainedDone"),
    );
  };

  const confirmBatch = (person: QuestionGroup, questions: ReviewQuestion[]) => {
    const confirm = questions.filter((q) => !unchecked.has(q.id)).map((q) => q.id);
    const reject = questions.filter((q) => unchecked.has(q.id)).map((q) => q.id);
    void run(
      (override) => commands.answerQuestions(person.id, confirm, reject, override),
      (o) => t("people.queue.mergedDone", { count: o.mergedGroups, name: name(person) }),
    );
  };

  const onKeyDown = (e: KeyboardEvent<HTMLDivElement>) => {
    if (!item || busy || e.ctrlKey || e.metaKey || e.altKey) return;
    if ((e.target as HTMLElement).closest("input, textarea, select")) {
      if (e.key !== "Enter") return;
    }
    const key = e.key.toLowerCase();
    let handled = true;
    if (key === "arrowright") setSkipped((s) => s + 1);
    else if (item.kind === "batch") {
      if (key === "enter") confirmBatch(item.person, item.questions);
      else handled = false;
    } else {
      const q = item.question;
      if (q.kind === "group_choice" && q.second) {
        if (key === "1") answer(q, { kind: "choose", person: q.other.id });
        else if (key === "2") answer(q, { kind: "choose", person: q.second.id });
        else if (key === "n") answer(q, { kind: "neither" });
        else handled = false;
      } else if (key === "s" || key === "y") answer(q, { kind: "yes" });
      else if (key === "n") answer(q, { kind: "no" });
      else handled = false;
    }
    if (handled) {
      // Le scorciatoie del modulo (N, I…) non devono agire anche sulla scheda.
      e.preventDefault();
      e.stopPropagation();
    }
  };

  const open = queue.data?.open ?? 0;
  return (
    <div
      ref={panelRef}
      tabIndex={-1}
      onKeyDown={onKeyDown}
      className="people-queue people-stack people-stack--gap-md people-pad"
      aria-label={t("people.queue.title")}
      role="region"
    >
      <div className="people-inline">
        <h2 className="people-title">{t("people.queue.title")}</h2>
        {open > 0 && (
          <span className="muted people-muted-sm">
            {t("people.queue.position", { n: index + 1, total: open })}
          </span>
        )}
        {items.length > 1 && (
          <Button
            size="sm"
            variant="ghost"
            className="people-ml-auto"
            onClick={() => setSkipped((s) => s + 1)}
          >
            {t("people.queue.skip")}
          </Button>
        )}
      </div>
      {queue.isError && (
        <InlineAlert tone="danger" title={t("people.failed")}>
          {errorText(queue.error)}
        </InlineAlert>
      )}
      {queue.data && !item && (
        <p role="status" className="muted people-muted-reset">
          {t("people.queue.empty", { count: queue.data.unnamedGroups })}
        </p>
      )}
      {conflict && (
        <InlineAlert tone="warn" title={t("people.queue.conflictTitle")}>
          <p className="people-muted-reset">{t("people.queue.conflictBody")}</p>
          <div className="people-question-actions">
            <Button variant="primary" disabled={busy} onClick={conflict.retry}>
              {t("people.queue.conflictOverride")}
            </Button>
            <Button disabled={busy} onClick={() => setConflict(null)}>
              {t("common.cancel")}
            </Button>
          </div>
        </InlineAlert>
      )}
      {error && (
        <InlineAlert tone="danger" title={t("people.failed")} onDismiss={() => setError("")}>
          {error}
        </InlineAlert>
      )}
      {item?.kind === "single" && (
        <SingleQuestion
          question={item.question}
          busy={busy}
          onAnswer={(a) => answer(item.question, a)}
        />
      )}
      {item?.kind === "batch" && (
        <BatchQuestion
          person={item.person}
          questions={item.questions}
          busy={busy}
          unchecked={unchecked}
          onToggle={(id) =>
            setUnchecked((s) => {
              const next = new Set(s);
              if (next.has(id)) next.delete(id);
              else next.add(id);
              return next;
            })
          }
          onConfirm={() => confirmBatch(item.person, item.questions)}
        />
      )}
    </div>
  );
}
