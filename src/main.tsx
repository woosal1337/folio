import React from "react";
import ReactDOM from "react-dom/client";

import App from "@/App";
import { applyInitialTheme } from "@/shared/hooks/use-theme";
import { applyInitialReadingControls } from "@/shared/hooks/use-reading-controls";
import "@/styles/globals.css";

applyInitialTheme();
applyInitialReadingControls();

const rootElement = document.getElementById("root");
if (!rootElement) {
  throw new Error("Root element #root not found in index.html");
}

ReactDOM.createRoot(rootElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>
);
