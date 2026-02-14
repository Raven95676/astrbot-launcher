import React from 'react';
import ReactDOM from 'react-dom/client';
import { attachConsole } from '@tauri-apps/plugin-log';
import { api } from './api';
import App from './App';

void attachConsole();

const isMacOS = await api.isMacOS();

ReactDOM.createRoot(document.getElementById('root') as HTMLElement).render(
  <React.StrictMode>
    <App isMacOS={isMacOS} />
  </React.StrictMode>
);
