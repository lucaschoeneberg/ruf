import { mount } from "svelte";
import { isTauri } from "@tauri-apps/api/core";
import App from "./App.svelte";
import Settings from "./Settings.svelte";
import "./app.css";

const target = document.getElementById("app");
if (!target) throw new Error("Application mount element is missing.");

const settingsWindow =
  new URLSearchParams(window.location.search).get("window") === "settings";
document.documentElement.classList.toggle("settings-document", settingsWindow);
document.documentElement.classList.toggle("native-document", isTauri());
mount(settingsWindow ? Settings : App, { target });
