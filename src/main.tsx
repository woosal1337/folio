import React from "react";
import ReactDOM from "react-dom/client";

import App from "@/App";
import { applyInitialTheme } from "@/hooks/use-theme";
import "@/styles/globals.css";

applyInitialTheme();

const rootElement = document.getElementById("root");
if (!rootElement) {
  throw new Error("Root element #root not found in index.html");
}

ReactDOM.createRoot(rootElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>
);
