import React from "react";
import ReactDOM from "react-dom/client";
import { ThemeProvider } from "./context/ThemeContext";
import "./i18n";
import App from "./App";

// Disable the native browser context menu in the Tauri app.
// Custom context menus (e.g. <ContextMenu>) handle their own onContextMenu
// events and call stopPropagation(), so they continue to work.
document.addEventListener("contextmenu", (event) => {
  event.preventDefault();
});

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <ThemeProvider>
      <App />
    </ThemeProvider>
  </React.StrictMode>,
);
