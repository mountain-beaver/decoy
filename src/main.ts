import '@fontsource-variable/onest/wght.css';
import '@fontsource-variable/bricolage-grotesque/wght.css';
import './app.css';
import { mount } from 'svelte';
import App from './App.svelte';
import { app } from './lib/store.svelte';

app.init();
mount(App, { target: document.getElementById('app')! });

// In browser mode (the mock) the state is reachable from the console for debugging.
if (!('__TAURI_INTERNALS__' in window)) (window as unknown as { __decoy: typeof app }).__decoy = app;
