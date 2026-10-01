import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { CaptureApp } from "./features/capture/CaptureApp";
import "./styles/theme.css";

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <CaptureApp />
  </StrictMode>,
);
