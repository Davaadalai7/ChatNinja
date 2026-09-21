import React from "react";
import { createRoot } from "react-dom/client";
import { App } from "./App";
import { Overlay } from "./features/overlay/Overlay";
import "./styles.css";

const overlay = new URLSearchParams(location.search).has("overlay");
document.body.classList.toggle("overlay-page", overlay);
createRoot(document.getElementById("root")!).render(
  <React.StrictMode>{overlay ? <Overlay /> : <App />}</React.StrictMode>,
);
