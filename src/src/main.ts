import { invoke } from "@tauri-apps/api/core";
import { mount } from "svelte";
import "./lib/styles/global.css";
import App from "./App.svelte";

function report(message: string, detail?: string) {
  void invoke('report_client_error', { message, detail: detail ?? null }).catch(() => undefined)
}

window.addEventListener('error', (event) => report(event.message, event.error instanceof Error ? event.error.stack : undefined))
window.addEventListener('unhandledrejection', (event) => {
  const reason: unknown = event.reason
  report(reason instanceof Error ? reason.message : String(reason), reason instanceof Error ? reason.stack : undefined)
})

const app = mount(App, {
  target: document.getElementById("app")!,
});

export default app;
