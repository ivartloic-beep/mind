import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { PanelApp } from "./features/tasks/PanelApp";
import "./styles/theme.css";

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <PanelApp />
  </StrictMode>,
);
