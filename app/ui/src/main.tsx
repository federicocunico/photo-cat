import React from "react";
import ReactDOM from "react-dom/client";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import "@fontsource/source-sans-3/400.css";
import "@fontsource/source-sans-3/600.css";
import "@fontsource/source-sans-3/700.css";
import "@fontsource/jetbrains-mono/400.css";
import "@fontsource/jetbrains-mono/500.css";
import "./styles/global.css";
import "./i18n";
import { applyTheme, type ThemeName } from "./theme/tokens";

// Tema salvato già per la schermata di avvio (le preferenze complete arrivano dopo).
function savedTheme(): ThemeName {
  try {
    const raw = localStorage.getItem("photo-cat.layout");
    const theme = raw ? (JSON.parse(raw) as { state?: { theme?: string } }).state?.theme : null;
    if (theme === "light" || theme === "dark") return theme;
  } catch {
    // Preferenze illeggibili: si usa il tema del sistema.
  }
  return window.matchMedia?.("(prefers-color-scheme: light)").matches ? "light" : "dark";
}
applyTheme(savedTheme());

const queryClient = new QueryClient({
  defaultOptions: { queries: { staleTime: 1000, retry: 1 } },
});

import { App } from "./App";

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <QueryClientProvider client={queryClient}>
      <App />
    </QueryClientProvider>
  </React.StrictMode>,
);
