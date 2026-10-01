import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { PostItApp } from "./features/postit/PostItApp";
import "./styles/theme.css";

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <PostItApp />
  </StrictMode>,
);
