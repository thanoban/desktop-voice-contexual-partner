// VoicePartner — Local-first voice companion desktop application
// Copyright (C) 2026 thanoban
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.

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
