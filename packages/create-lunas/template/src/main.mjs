import { attach } from "lunas";
import App from "./App.lunas";

// A compiled Lunas component's default export is a factory: call it with props
// to build a detached root, then `attach` it to a host element in the DOM.
attach(App(), document.getElementById("app"));
