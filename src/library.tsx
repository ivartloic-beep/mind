import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { LibraryApp } from "./features/library/LibraryApp";
import "./styles/theme.css";

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <LibraryApp />
  </StrictMode>,
);
