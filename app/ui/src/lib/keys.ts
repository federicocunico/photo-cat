// Utilità condivise dai gestori della tastiera.

const NON_TEXT_INPUTS = [
  "checkbox",
  "radio",
  "range",
  "button",
  "submit",
  "reset",
  "color",
  "file",
];

/** Vero se il fuoco è in un campo dove i tasti scrivono testo. */
export function isTyping(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) return false;
  if (target.isContentEditable || target.closest("[contenteditable='true']")) return true;
  const tag = target.tagName;
  if (tag === "TEXTAREA" || tag === "SELECT") return true;
  if (tag === "INPUT") return !NON_TEXT_INPUTS.includes((target as HTMLInputElement).type);
  return false;
}

/** Vero se è aperta una finestra modale, che tiene per sé la tastiera. */
export function modalOpen(): boolean {
  return document.querySelector('[role="dialog"][aria-modal="true"], [role="alertdialog"]') != null;
}
