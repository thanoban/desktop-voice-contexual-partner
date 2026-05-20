import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import { Widget } from "./Widget";
import "./styles.css";

const isWidget = new URLSearchParams(window.location.search).has("widget");

if (isWidget) {
  document.documentElement.classList.add("widget-mode");
}

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    {isWidget ? <Widget /> : <App />}
  </React.StrictMode>
);
