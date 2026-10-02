import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { HomePrototype } from "./HomePrototype";
import "./prototype.css";

createRoot(document.getElementById("root")!).render(<StrictMode><HomePrototype /></StrictMode>);
