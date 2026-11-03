// Adattatore delle icone: l'app usa solo `Icon.*`, così il set (oggi lucide-react, ISC)
// si cambia in un punto solo. Le icone sono decorative (aria-hidden): il nome
// accessibile lo dà sempre il controllo che le contiene.

import type { SVGProps } from "react";
import {
  Activity,
  ArrowRight,
  ArrowUp,
  Check,
  ChevronDown,
  ChevronLeft,
  ChevronRight,
  CircleHelp,
  Copy,
  Cpu,
  Download,
  Ellipsis,
  ExternalLink,
  Filter,
  FolderInput,
  FolderTree,
  GalleryHorizontalEnd,
  Images,
  Info,
  Keyboard,
  Layers,
  LoaderCircle,
  MemoryStick,
  Minus,
  PanelLeft,
  PanelRight,
  Pause,
  Pencil,
  Play,
  Plus,
  RefreshCw,
  Search,
  Settings,
  Sparkles,
  Square,
  SquareUserRound,
  Stethoscope,
  Tags,
  Trash2,
  TriangleAlert,
  Undo2,
  Users,
  X,
  type LucideIcon,
} from "lucide-react";

export type IconProps = Omit<SVGProps<SVGSVGElement>, "ref"> & { size?: number };

function wrap(C: LucideIcon) {
  const Wrapped = ({ size = 18, strokeWidth = 1.8, ...rest }: IconProps) => (
    <C size={size} strokeWidth={strokeWidth} aria-hidden="true" focusable="false" {...rest} />
  );
  Wrapped.displayName = `Icon(${C.displayName ?? "lucide"})`;
  return Wrapped;
}

/** Marchio di photo-cat: unico disegno proprio. */
function Logo({ size = 18, ...rest }: IconProps) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth={1.8}
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
      focusable="false"
      {...rest}
    >
      <circle cx="12" cy="12" r="9" />
      <path d="M12 3l3 7M21 12l-7 3M12 21l-3-7M3 12l7-3" />
    </svg>
  );
}

export const Icon = {
  Logo,
  // Moduli
  Library: wrap(Images),
  Catalogs: wrap(Layers),
  People: wrap(Users),
  Identities: wrap(SquareUserRound),
  Sequence: wrap(GalleryHorizontalEnd),
  Tags: wrap(Tags),
  Import: wrap(FolderInput),
  Export: wrap(Download),
  Organize: wrap(FolderTree),
  Jobs: wrap(Activity),
  Models: wrap(Cpu),
  Diagnostics: wrap(Stethoscope),
  Settings: wrap(Settings),
  // Stato
  Spinner: wrap(LoaderCircle),
  Ram: wrap(MemoryStick),
  Alert: wrap(TriangleAlert),
  Info: wrap(Info),
  Smart: wrap(Sparkles),
  Check: wrap(Check),
  // Comandi
  Search: wrap(Search),
  Filter: wrap(Filter),
  ChevronLeft: wrap(ChevronLeft),
  ChevronRight: wrap(ChevronRight),
  ChevronDown: wrap(ChevronDown),
  ArrowUp: wrap(ArrowUp),
  ArrowRight: wrap(ArrowRight),
  Pause: wrap(Pause),
  Play: wrap(Play),
  Stop: wrap(Square),
  Close: wrap(X),
  Edit: wrap(Pencil),
  Refresh: wrap(RefreshCw),
  Plus: wrap(Plus),
  Minus: wrap(Minus),
  More: wrap(Ellipsis),
  Copy: wrap(Copy),
  Delete: wrap(Trash2),
  Undo: wrap(Undo2),
  OpenExternal: wrap(ExternalLink),
  Help: wrap(CircleHelp),
  Keyboard: wrap(Keyboard),
  PanelLeft: wrap(PanelLeft),
  PanelRight: wrap(PanelRight),
};

export type IconName = keyof typeof Icon;
