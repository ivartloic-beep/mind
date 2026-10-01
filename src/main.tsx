import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import "./styles/theme.css";

/**
 * Host / shell principal — fenêtre cachée (tray + raccourcis en étapes 11–12).
 * Étape 1 : stub minimal.
 */
function MainHost() {
  return (
    <main className="shell">
      <span className="badge">main</span>
      <h1>Ma Tête</h1>
      <p>Host invisible — tray et raccourcis arriveront aux étapes 11–12.</p>
    </main>
  );
}

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <MainHost />
  </StrictMode>,
);
