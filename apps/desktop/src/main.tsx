import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { App } from "./App";
import "./styles.css";
import {SpotlightViewerHost} from "./app/collection/SpotlightViewer";
const detached = new URLSearchParams(window.location.search).has("spotlight");

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    {detached ? <SpotlightViewerHost detached/> : <><App /><SpotlightViewerHost/></>}
  </StrictMode>,
);
