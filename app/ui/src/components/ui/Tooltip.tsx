import * as RT from "@radix-ui/react-tooltip";
import type { ReactElement, ReactNode } from "react";

/**
 * Suggerimento al passaggio e al focus. Non sostituisce il nome accessibile del controllo:
 * è un aiuto visivo in più (Esc lo chiude). Ogni suggerimento ha il proprio provider, così
 * il componente funziona ovunque, anche nei test, senza un contenitore globale.
 */
export function Tooltip({
  content,
  children,
  side = "top",
}: {
  content: ReactNode;
  children: ReactElement;
  side?: "top" | "right" | "bottom" | "left";
}) {
  return (
    <RT.Provider delayDuration={500} skipDelayDuration={300}>
      <RT.Root>
        <RT.Trigger asChild>{children}</RT.Trigger>
        <RT.Portal>
          <RT.Content className="ui-tooltip" side={side} sideOffset={6} collisionPadding={8}>
            {content}
          </RT.Content>
        </RT.Portal>
      </RT.Root>
    </RT.Provider>
  );
}
