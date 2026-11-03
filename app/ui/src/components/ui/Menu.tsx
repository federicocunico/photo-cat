// Menu a comparsa, menu contestuale e popover (Radix): frecce, Invio, Esc, ritorno del focus.
import * as RContext from "@radix-ui/react-context-menu";
import * as RDropdown from "@radix-ui/react-dropdown-menu";
import * as RPopover from "@radix-ui/react-popover";
import type { ReactElement, ReactNode } from "react";

export type MenuItem =
  | {
      kind?: "item";
      label: ReactNode;
      icon?: ReactNode;
      /** Scorciatoia mostrata a destra (solo testo: il tasto lo gestisce il registro). */
      shortcut?: string;
      danger?: boolean;
      disabled?: boolean;
      onSelect: () => unknown;
    }
  | { kind: "separator" }
  | { kind: "label"; label: ReactNode }
  | { kind: "submenu"; label: ReactNode; icon?: ReactNode; items: MenuItem[] };

type Parts = {
  Item: typeof RDropdown.Item;
  Separator: typeof RDropdown.Separator;
  Label: typeof RDropdown.Label;
  Sub: typeof RDropdown.Sub;
  SubTrigger: typeof RDropdown.SubTrigger;
  SubContent: typeof RDropdown.SubContent;
  Portal: typeof RDropdown.Portal;
};

function renderItems(items: MenuItem[], P: Parts): ReactNode {
  return items.map((it, i) => {
    if (it.kind === "separator") return <P.Separator key={i} className="ui-menu-separator" />;
    if (it.kind === "label")
      return (
        <P.Label key={i} className="ui-menu-label">
          {it.label}
        </P.Label>
      );
    if (it.kind === "submenu")
      return (
        <P.Sub key={i}>
          <P.SubTrigger className="ui-menu-item">
            {it.icon}
            <span className="ui-menu-text">{it.label}</span>
            <span aria-hidden="true">›</span>
          </P.SubTrigger>
          <P.Portal>
            <P.SubContent className="ui-menu" sideOffset={4} collisionPadding={8}>
              {renderItems(it.items, P)}
            </P.SubContent>
          </P.Portal>
        </P.Sub>
      );
    return (
      <P.Item
        key={i}
        className={["ui-menu-item", it.danger ? "ui-menu-danger" : ""].filter(Boolean).join(" ")}
        disabled={it.disabled}
        onSelect={() => void it.onSelect()}
      >
        {it.icon}
        <span className="ui-menu-text">{it.label}</span>
        {it.shortcut && <span className="ui-menu-shortcut">{it.shortcut}</span>}
      </P.Item>
    );
  });
}

/** Menu aperto da un pulsante: il menu prende il nome del pulsante (il figlio). */
export function Menu({
  trigger,
  items,
  align = "start",
}: {
  trigger: ReactElement;
  items: MenuItem[];
  align?: "start" | "end";
}) {
  return (
    <RDropdown.Root modal={false}>
      <RDropdown.Trigger asChild>{trigger}</RDropdown.Trigger>
      <RDropdown.Portal>
        <RDropdown.Content className="ui-menu" align={align} sideOffset={4} collisionPadding={8}>
          {renderItems(items, RDropdown)}
        </RDropdown.Content>
      </RDropdown.Portal>
    </RDropdown.Root>
  );
}

/** Menu contestuale (clic destro o tasto Menu / Maiusc+F10) sul figlio. */
export function ContextMenu({
  children,
  items,
  label,
  onOpenChange,
}: {
  children: ReactElement;
  items: MenuItem[];
  label?: string;
  onOpenChange?: (open: boolean) => void;
}) {
  return (
    <RContext.Root modal={false} onOpenChange={onOpenChange}>
      <RContext.Trigger asChild>{children}</RContext.Trigger>
      <RContext.Portal>
        <RContext.Content className="ui-menu" aria-label={label} collisionPadding={8}>
          {renderItems(items, RContext as unknown as Parts)}
        </RContext.Content>
      </RContext.Portal>
    </RContext.Root>
  );
}

/** Pannello non modale ancorato a un pulsante (selettori con ricerca, dettagli). */
export function Popover({
  trigger,
  children,
  label,
  open,
  onOpenChange,
  align = "start",
}: {
  trigger: ReactElement;
  children: ReactNode;
  /** Nome del pannello per il lettore di schermo. */
  label: string;
  open?: boolean;
  onOpenChange?: (open: boolean) => void;
  align?: "start" | "center" | "end";
}) {
  return (
    <RPopover.Root open={open} onOpenChange={onOpenChange}>
      <RPopover.Trigger asChild>{trigger}</RPopover.Trigger>
      <RPopover.Portal>
        <RPopover.Content
          className="ui-popover"
          aria-label={label}
          role="dialog"
          align={align}
          sideOffset={6}
          collisionPadding={8}
        >
          {children}
        </RPopover.Content>
      </RPopover.Portal>
    </RPopover.Root>
  );
}
