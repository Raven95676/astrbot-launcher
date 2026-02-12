// Global holder for antd static functions obtained via App.useApp().
// This allows hooks, utils, and other non-component code to use
// context-aware message/notification/modal without prop drilling.

import { useEffect } from 'react';
import { App } from 'antd';
import type { MessageInstance } from 'antd/es/message/interface';

const messageHolder: { current: MessageInstance | null } = { current: null };

/**
 * Component that captures App.useApp() static functions into module-level variables.
 * Must be rendered inside an antd <App> component.
 */
export function AntdStaticProvider() {
  const app = App.useApp();
  useEffect(() => {
    messageHolder.current = app.message;
  }, [app.message]);
  return null;
}

export const message: MessageInstance = new Proxy({} as MessageInstance, {
  get(_target, prop) {
    const instance = messageHolder.current;
    if (!instance) {
      throw new Error(
        'AntdStaticProvider not mounted. Ensure <AntdStaticProvider /> is rendered inside <App>.'
      );
    }
    const value = instance[prop as keyof MessageInstance];
    return typeof value === 'function' ? value.bind(instance) : value;
  },
});
