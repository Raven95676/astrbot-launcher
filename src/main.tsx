import React from 'react';
import ReactDOM from 'react-dom/client';
import { attachConsole } from '@tauri-apps/plugin-log';
import App from './App';

// Attach console to receive Rust logs in browser devtools
attachConsole();

ReactDOM.createRoot(document.getElementById('root') as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>
);
