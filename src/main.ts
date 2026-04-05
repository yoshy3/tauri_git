import { mount } from 'svelte'
import App from "./App.svelte";
import "./app.css";
import { setupI18n } from "./lib/i18n";

setupI18n();

const app = mount(App, {
  target: document.getElementById('app')!,
})

export default app;
