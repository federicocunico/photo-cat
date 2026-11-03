// Controlli di scelta e struttura: controllo segmentato, schede, sezioni richiudibili.
import * as RCollapsible from "@radix-ui/react-collapsible";
import * as RTabs from "@radix-ui/react-tabs";
import * as RToggleGroup from "@radix-ui/react-toggle-group";
import { useState, type ReactNode } from "react";
import { Icon } from "../icons";

export type Option<V extends string> = {
  value: V;
  label: ReactNode;
  icon?: ReactNode;
  title?: string;
};

/** Scelta tra 2–4 opzioni sempre visibili (frecce per spostarsi, una sola attiva). */
export function SegmentedControl<V extends string>({
  label,
  value,
  options,
  onChange,
  disabled,
}: {
  /** Nome del gruppo per il lettore di schermo. */
  label: string;
  value: V;
  options: Option<V>[];
  onChange: (value: V) => void;
  disabled?: boolean;
}) {
  return (
    <RToggleGroup.Root
      type="single"
      className="ui-segmented"
      aria-label={label}
      value={value}
      disabled={disabled}
      onValueChange={(v) => {
        // Radix permette di deselezionare: qui una scelta resta sempre attiva.
        if (v) onChange(v as V);
      }}
    >
      {options.map((o) => (
        <RToggleGroup.Item key={o.value} value={o.value} className="ui-segment" title={o.title}>
          {o.icon}
          {o.label}
        </RToggleGroup.Item>
      ))}
    </RToggleGroup.Root>
  );
}

/** Schede: frecce per spostarsi, il contenuto segue la scheda attiva. */
export function Tabs<V extends string>({
  label,
  value,
  onChange,
  tabs,
}: {
  label: string;
  value: V;
  onChange: (value: V) => void;
  tabs: Array<{ value: V; label: ReactNode; content: ReactNode }>;
}) {
  return (
    <RTabs.Root value={value} onValueChange={(v) => onChange(v as V)} className="ui-tabs">
      <RTabs.List aria-label={label} className="ui-tab-list">
        {tabs.map((tab) => (
          <RTabs.Trigger key={tab.value} value={tab.value} className="ui-tab">
            {tab.label}
          </RTabs.Trigger>
        ))}
      </RTabs.List>
      {tabs.map((tab) => (
        <RTabs.Content key={tab.value} value={tab.value} className="ui-tab-panel">
          {tab.content}
        </RTabs.Content>
      ))}
    </RTabs.Root>
  );
}

/**
 * Sezione richiudibile. Controllata (`open` + `onOpenChange`, per persistere lo stato)
 * o libera (`defaultOpen`).
 */
export function Disclosure({
  title,
  children,
  open,
  defaultOpen = true,
  onOpenChange,
  actions,
  level = 3,
}: {
  title: ReactNode;
  children: ReactNode;
  open?: boolean;
  defaultOpen?: boolean;
  onOpenChange?: (open: boolean) => void;
  /** Comandi accanto al titolo, fuori dal pulsante (niente controlli annidati). */
  actions?: ReactNode;
  level?: 2 | 3 | 4;
}) {
  const [inner, setInner] = useState(defaultOpen);
  const isOpen = open ?? inner;
  const H = `h${level}` as "h2" | "h3" | "h4";
  return (
    <RCollapsible.Root
      open={isOpen}
      onOpenChange={(o) => {
        if (open === undefined) setInner(o);
        onOpenChange?.(o);
      }}
      className="ui-disclosure"
    >
      <div className="ui-disclosure-head">
        <H>
          <RCollapsible.Trigger className="ui-disclosure-trigger">
            <span className="ui-disclosure-chevron" data-open={isOpen || undefined}>
              <Icon.ChevronRight size={14} />
            </span>
            {title}
          </RCollapsible.Trigger>
        </H>
        {actions}
      </div>
      <RCollapsible.Content className="ui-disclosure-content">{children}</RCollapsible.Content>
    </RCollapsible.Root>
  );
}
