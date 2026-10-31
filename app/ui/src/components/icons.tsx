// Icone a tratto (stile dei mockup). Decorative: aria-hidden.

import type { SVGProps } from "react";

type P = SVGProps<SVGSVGElement> & { size?: number };

function Svg({ size = 18, children, ...rest }: P) {
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
      {...rest}
    >
      {children}
    </svg>
  );
}

export const Icon = {
  Logo: (p: P) => (
    <Svg {...p}>
      <circle cx="12" cy="12" r="9" />
      <path d="M12 3l3 7M21 12l-7 3M12 21l-3-7M3 12l7-3" />
    </Svg>
  ),
  Search: (p: P) => (
    <Svg {...p}>
      <circle cx="11" cy="11" r="7" />
      <path d="M20 20l-4-4" />
    </Svg>
  ),
  Library: (p: P) => (
    <Svg {...p}>
      <rect x="3" y="3" width="7" height="7" />
      <rect x="14" y="3" width="7" height="7" />
      <rect x="3" y="14" width="7" height="7" />
      <rect x="14" y="14" width="7" height="7" />
    </Svg>
  ),
  Catalogs: (p: P) => (
    <Svg {...p}>
      <path d="M3 7l9-4 9 4-9 4-9-4z" />
      <path d="M3 12l9 4 9-4" />
      <path d="M3 17l9 4 9-4" />
    </Svg>
  ),
  People: (p: P) => (
    <Svg {...p}>
      <circle cx="9" cy="8" r="3.5" />
      <path d="M2.5 20c0-3.6 2.9-6 6.5-6s6.5 2.4 6.5 6" />
      <circle cx="17" cy="9" r="2.5" />
      <path d="M16 14.2c3 .2 5.5 2.4 5.5 5.8" />
    </Svg>
  ),
  Import: (p: P) => (
    <Svg {...p}>
      <path d="M12 3v12" />
      <path d="M7 10l5 5 5-5" />
      <path d="M4 19h16" />
    </Svg>
  ),
  Jobs: (p: P) => (
    <Svg {...p}>
      <path d="M4 6h16M4 12h10M4 18h6" />
    </Svg>
  ),
  Models: (p: P) => (
    <Svg {...p}>
      <rect x="6" y="6" width="12" height="12" rx="2" />
      <path d="M9 2v4M15 2v4M9 18v4M15 18v4M2 9h4M2 15h4M18 9h4M18 15h4" />
    </Svg>
  ),
  Diagnostics: (p: P) => (
    <Svg {...p}>
      <path d="M3 12h4l3-7 4 14 3-7h4" />
    </Svg>
  ),
  Organize: (p: P) => (
    <Svg {...p}>
      <path d="M3 6h7l2 2h9v11H3z" />
      <path d="M12 12v5M9.5 14.5h5" />
    </Svg>
  ),
  Settings: (p: P) => (
    <Svg {...p}>
      <circle cx="12" cy="12" r="3" />
      <path d="M12 2v3M12 19v3M2 12h3M19 12h3M4.9 4.9l2.1 2.1M17 17l2.1 2.1M4.9 19.1L7 17M17 7l2.1-2.1" />
    </Svg>
  ),
  Spinner: (p: P) => (
    <Svg strokeWidth={2.4} {...p}>
      <path d="M21 12a9 9 0 1 1-9-9" />
    </Svg>
  ),
  Ram: (p: P) => (
    <Svg strokeWidth={2} {...p}>
      <rect x="3" y="7" width="18" height="10" rx="1.5" />
      <path d="M7 7v10M11 7v10M15 7v10" />
    </Svg>
  ),
  ChevronLeft: (p: P) => (
    <Svg {...p}>
      <path d="M15 6l-6 6 6 6" />
    </Svg>
  ),
  ChevronRight: (p: P) => (
    <Svg {...p}>
      <path d="M9 6l6 6-6 6" />
    </Svg>
  ),
  Pause: (p: P) => (
    <Svg {...p}>
      <path d="M8 5v14M16 5v14" />
    </Svg>
  ),
  Play: (p: P) => (
    <Svg {...p}>
      <path d="M7 5l12 7-12 7z" />
    </Svg>
  ),
  Stop: (p: P) => (
    <Svg {...p}>
      <rect x="6" y="6" width="12" height="12" rx="1" />
    </Svg>
  ),
  Smart: (p: P) => (
    <Svg strokeWidth={2.2} {...p}>
      <path d="M13 2L4 14h7l-1 8 9-12h-7z" />
    </Svg>
  ),
  Alert: (p: P) => (
    <Svg {...p}>
      <path d="M12 3l10 18H2z" />
      <path d="M12 10v5M12 18v.5" />
    </Svg>
  ),
};
