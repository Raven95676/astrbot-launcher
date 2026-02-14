import { useEffect, useState } from 'react';
import { MinusOutlined, BorderOutlined, BlockOutlined, CloseOutlined } from '@ant-design/icons';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { api } from '../api';

export function TitleBar() {
  const [isMacOS, setIsMacOS] = useState<boolean | null>(null);

  useEffect(() => {
    let mounted = true;
    api
      .isMacOS()
      .then((value) => {
        if (mounted) {
          setIsMacOS(value);
        }
      })
      .catch(() => {
        if (mounted) {
          setIsMacOS(false);
        }
      });

    return () => {
      mounted = false;
    };
  }, []);

  if (isMacOS === null) return null;
  if (isMacOS) return null;

  return <TitleBarInner />;
}

function TitleBarInner() {
  const [maximized, setMaximized] = useState(false);
  const appWindow = getCurrentWindow();

  useEffect(() => {
    appWindow.isMaximized().then(setMaximized);

    const unlisten = appWindow.onResized(async () => {
      setMaximized(await appWindow.isMaximized());
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, [appWindow]);

  return (
    <div className="titlebar" data-tauri-drag-region>
      <div className="titlebar-title" data-tauri-drag-region>
        AstrBot Launcher
      </div>
      <div className="titlebar-controls">
        <button className="titlebar-btn" aria-label="Minimize" onClick={() => appWindow.minimize()}>
          <MinusOutlined style={{ fontSize: 12 }} />
        </button>
        <button
          className="titlebar-btn"
          aria-label={maximized ? 'Restore' : 'Maximize'}
          onClick={() => appWindow.toggleMaximize()}
        >
          {maximized ? (
            <BlockOutlined style={{ fontSize: 12 }} />
          ) : (
            <BorderOutlined style={{ fontSize: 12 }} />
          )}
        </button>
        <button
          className="titlebar-btn titlebar-btn-close"
          aria-label="Close"
          onClick={() => appWindow.close()}
        >
          <CloseOutlined style={{ fontSize: 12 }} />
        </button>
      </div>
    </div>
  );
}
