// Barra dei menu (application-menu; ui-usability-audit D6): File, Modifica, Vista, Vai, Strumenti
// e Aiuto. Le voci vengono dal registro dei comandi, con lo stesso nome e la stessa scorciatoia
// della palette; una voce non disponibile resta visibile, disattivata, con il motivo.
// Su macOS gli stessi menu stanno nella barra del sistema e questo componente non si mostra.
import * as RMenubar from "@radix-ui/react-menubar";
import { useEffect, useId, useRef, useState, type KeyboardEvent, type ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { Icon } from "../icons";
import { isTyping, modalOpen } from "../../lib/keys";
import {
  commandEnabled,
  menuEntries,
  useCommandStore,
  type Command,
  type MenuEntry,
} from "../../commands/registry";
import { MENUS, accessKey, keysOf } from "./menuBarModel";

function Item({ command }: { command: Command }) {
  const reasonId = useId();
  const enabled = commandEnabled(command);
  const reason = enabled ? undefined : (command.disabledReason?.() ?? undefined);
  const keys = keysOf(command);
  const content = (
    <>
      <span className="menubar-label">
        {command.label}
        {reason && (
          <span id={reasonId} className="menubar-reason">
            {reason}
          </span>
        )}
      </span>
      {keys && <span className="menubar-keys">{keys}</span>}
    </>
  );
  // Disattivata ma raggiungibile da tastiera, per leggerne il motivo: niente `disabled` di Radix.
  const common = {
    className: "ui-menu-item menubar-item",
    "aria-disabled": enabled ? undefined : true,
    "aria-describedby": reason ? reasonId : undefined,
    title: reason,
    onSelect: (e: Event) => {
      if (!enabled) {
        e.preventDefault();
        return;
      }
      void command.run?.();
    },
  };
  if (command.checked)
    return (
      <RMenubar.CheckboxItem {...common} checked={command.checked()}>
        <RMenubar.ItemIndicator className="menubar-check">
          <Icon.Check size={14} />
        </RMenubar.ItemIndicator>
        {content}
      </RMenubar.CheckboxItem>
    );
  return <RMenubar.Item {...common}>{content}</RMenubar.Item>;
}

/** Voci di una sezione, con i sottomenu (es. "Tema") raggruppati sotto una voce. */
function Section({ entries }: { entries: MenuEntry[] }) {
  const out: ReactNode[] = [];
  const done = new Set<string>();
  for (const { command, placement } of entries) {
    const sub = placement.submenu;
    if (!sub) {
      out.push(<Item key={command.id} command={command} />);
      continue;
    }
    if (done.has(sub)) continue;
    done.add(sub);
    const items = entries.filter((e) => e.placement.submenu === sub);
    out.push(
      <RMenubar.Sub key={`sub:${sub}`}>
        <RMenubar.SubTrigger className="ui-menu-item menubar-item">
          <span className="menubar-label">{sub}</span>
          <Icon.ChevronRight size={14} aria-hidden="true" />
        </RMenubar.SubTrigger>
        <RMenubar.Portal>
          <RMenubar.SubContent className="ui-menu menubar-content" sideOffset={2} alignOffset={-4}>
            {items.map((e) => (
              <Item key={e.command.id} command={e.command} />
            ))}
          </RMenubar.SubContent>
        </RMenubar.Portal>
      </RMenubar.Sub>,
    );
  }
  return <>{out}</>;
}

export function MenuBar() {
  const { t } = useTranslation();
  // Il registro cambia quando i moduli montano o smontano i loro comandi.
  const commands = useCommandStore((s) => s.commands);
  const [open, setOpen] = useState("");
  /** Barra attivata da tastiera (Alt o F10): si vedono le lettere d'accesso. */
  const [keyboard, setKeyboard] = useState(false);
  const root = useRef<HTMLDivElement>(null);
  const returnFocus = useRef<HTMLElement | null>(null);
  const labels = MENUS.map((id) => ({ id, ...accessKey(t(`menu.names.${id}`)) }));

  const triggers = () => [
    ...(root.current?.querySelectorAll<HTMLElement>("[data-menu-trigger]") ?? []),
  ];
  const activate = () => {
    const first = triggers()[0];
    if (!first) return;
    const current = document.activeElement as HTMLElement | null;
    if (!root.current?.contains(current)) returnFocus.current = current;
    setKeyboard(true);
    first.focus();
  };
  const deactivate = () => {
    setKeyboard(false);
    setOpen("");
    const back = returnFocus.current;
    returnFocus.current = null;
    if (back && document.contains(back)) back.focus();
    else (document.activeElement as HTMLElement | null)?.blur();
  };

  // Alt premuto e rilasciato da solo, oppure F10: il focus va sul primo menu. Alt+tasto (es. le
  // valutazioni Alt+1…5) non attiva la barra. Alt+lettera d'accesso apre direttamente il menu.
  useEffect(() => {
    let altAlone = false;
    const down = (e: globalThis.KeyboardEvent) => {
      if (e.key === "Alt") {
        altAlone = !e.repeat && !e.ctrlKey && !e.metaKey && !e.shiftKey;
        return;
      }
      altAlone = false;
      if (e.key === "F10" && !e.shiftKey && !e.ctrlKey && !e.metaKey && !e.altKey) {
        e.preventDefault();
        activate();
        return;
      }
      if (
        e.altKey &&
        !e.ctrlKey &&
        !e.metaKey &&
        e.key.length === 1 &&
        !isTyping(e.target) &&
        !modalOpen()
      ) {
        const menu = labels.find((l) => l.key === e.key.toLowerCase());
        if (!menu || e.defaultPrevented) return;
        e.preventDefault();
        returnFocus.current = document.activeElement as HTMLElement | null;
        setKeyboard(true);
        setOpen(menu.id);
      }
    };
    const up = (e: globalThis.KeyboardEvent) => {
      if (e.key !== "Alt" || !altAlone) return;
      altAlone = false;
      if (isTyping(e.target) || modalOpen()) return;
      e.preventDefault();
      if (root.current?.contains(document.activeElement)) deactivate();
      else activate();
    };
    const blur = () => (altAlone = false);
    window.addEventListener("keydown", down);
    window.addEventListener("keyup", up);
    window.addEventListener("blur", blur);
    return () => {
      window.removeEventListener("keydown", down);
      window.removeEventListener("keyup", up);
      window.removeEventListener("blur", blur);
    };
  });

  // Barra attiva da tastiera: una lettera apre il menu corrispondente; Esc con i menu chiusi
  // riporta il focus dov'era.
  const onKeyDown = (e: KeyboardEvent<HTMLDivElement>) => {
    if (e.key === "Escape" && open === "") {
      e.preventDefault();
      deactivate();
      return;
    }
    if (open === "" && e.key.length === 1 && !e.ctrlKey && !e.metaKey && !e.altKey) {
      const menu = labels.find((l) => l.key === e.key.toLowerCase());
      if (menu) {
        e.preventDefault();
        setOpen(menu.id);
      }
    }
  };

  return (
    <RMenubar.Root
      ref={root}
      className="menubar"
      value={open}
      onValueChange={setOpen}
      aria-label={t("menu.bar")}
      data-keyboard={keyboard || undefined}
      onKeyDown={onKeyDown}
      onBlur={(e) => {
        const next = e.relatedTarget as Node | null;
        // Il focus passa al menu aperto (in un portale) o a un altro trigger: la barra resta attiva.
        if (
          next &&
          (root.current?.contains(next) || (next as Element).closest?.(".menubar-content"))
        )
          return;
        if (open === "") setKeyboard(false);
      }}
    >
      {labels.map(({ id, text, index }) => {
        const sections = menuEntries(id, commands);
        return (
          <RMenubar.Menu key={id} value={id}>
            <RMenubar.Trigger className="menubar-trigger" data-menu-trigger={id} aria-label={text}>
              {text.slice(0, index)}
              <span className="menubar-access">{text[index]}</span>
              {text.slice(index + 1)}
            </RMenubar.Trigger>
            <RMenubar.Portal>
              <RMenubar.Content
                className="ui-menu menubar-content"
                align="start"
                sideOffset={4}
                loop
                onCloseAutoFocus={(e) => {
                  // Chiudendo con un clic su una voce il focus torna dove l'utente lavorava.
                  if (!keyboard) e.preventDefault();
                }}
              >
                {sections.map((entries, i) => (
                  <div key={i} role="group" className="menubar-section">
                    {i > 0 && <RMenubar.Separator className="ui-menu-separator" />}
                    <Section entries={entries} />
                  </div>
                ))}
              </RMenubar.Content>
            </RMenubar.Portal>
          </RMenubar.Menu>
        );
      })}
    </RMenubar.Root>
  );
}
