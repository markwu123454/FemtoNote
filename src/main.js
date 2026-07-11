import { mount } from "svelte";
import "./app.css";
import App from "./App.svelte";
import ConfigWindow from "./windows/ConfigWindow.svelte";
import ExportWindow from "./windows/ExportWindow.svelte";

// Every window loads the same bundle; the URL hash picks the root component.
//   (no hash) -> main capture window
//   #setup    -> Config window, Subjects + Schedule
//   #settings -> Config window, Editor + Output
//   #export   -> Export window
const route = (window.location.hash || "").replace(/^#\/?/, "").trim();

let Root = App;
let props = {};
if (route === "setup") {
  Root = ConfigWindow;
  props = { section: "setup" };
} else if (route === "settings") {
  Root = ConfigWindow;
  props = { section: "settings" };
} else if (route === "export") {
  Root = ExportWindow;
}

// Svelte 5: components are mounted with `mount()`, not `new App(...)`.
const app = mount(Root, {
  target: document.getElementById("app"),
  props,
});

export default app;
