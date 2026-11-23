import { useTranslation } from "react-i18next";
import { Dialog } from "../ui";
import { useAboutDialog } from "../../store/menuIntent";
import { AboutPanel } from "./AboutPanel";

/** Finestra Informazioni su photo-cat: Aiuto › "Informazioni su photo-cat" (app-about). */
export function AboutDialog() {
  const { t } = useTranslation();
  const open = useAboutDialog((s) => s.open);
  const setOpen = useAboutDialog((s) => s.setOpen);
  return (
    <Dialog open={open} onOpenChange={setOpen} title={t("about.title")} width={640}>
      {open && <AboutPanel />}
    </Dialog>
  );
}
