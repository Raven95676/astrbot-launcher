import { create } from 'zustand';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { api } from '../api';
import { message } from '../antdStatic';
import type {
  InstalledVersion,
  InstanceStatus,
  AppConfig,
  AppSnapshot,
  BackupInfo,
  DeployProgress,
  DeployState,
} from '../types';
import { getErrorMessage } from '../utils';
import { MODAL_CLOSE_DELAY_MS } from '../constants';

interface AppState {
  // Data
  instances: InstanceStatus[];
  versions: InstalledVersion[];
  backups: BackupInfo[];
  pythonInstalled: boolean;
  config: AppConfig | null;
  loading: boolean;
  initialized: boolean;

  // Operations loading map
  operations: Record<string, boolean>;

  // Deploy state
  deployState: DeployState | null;

  // Actions
  hydrateSnapshot: (snapshot: AppSnapshot) => void;
  refresh: () => Promise<void>;
  reloadSnapshot: (options?: { throwOnError?: boolean }) => Promise<void>;
  rebuildSnapshotFromDisk: (options?: { throwOnError?: boolean }) => Promise<void>;
  startOperation: (key: string) => void;
  finishOperation: (key: string) => void;
  isOperationActive: (key: string) => boolean;
  startDeploy: (instanceName: string, type: 'start' | 'upgrade' | 'downgrade') => void;
  setDeployProgress: (progress: DeployProgress | null) => void;
  closeDeploy: () => void;
}

export const useAppStore = create<AppState>((set, get) => ({
  // Initial state
  instances: [],
  versions: [],
  backups: [],
  pythonInstalled: false,
  config: null,
  loading: false,
  initialized: false,
  operations: {},
  deployState: null,

  hydrateSnapshot: (snapshot: AppSnapshot) => {
    set({
      instances: snapshot.instances,
      versions: snapshot.versions,
      backups: snapshot.backups,
      pythonInstalled: snapshot.python_installed,
      config: snapshot.config,
      initialized: true,
    });
  },

  // Actions
  refresh: async () => {
    set({ loading: true });
    try {
      const snapshot = await api.getAppSnapshot();
      get().hydrateSnapshot(snapshot);
    } catch (e: unknown) {
      const msg = getErrorMessage(e);
      if (message?.error) {
        message.error(msg);
      } else {
        console.error(msg);
      }
    }
    set({ loading: false });
  },

  reloadSnapshot: async (options?: { throwOnError?: boolean }) => {
    set({ loading: true });
    try {
      const snapshot = await api.getAppSnapshot();
      get().hydrateSnapshot(snapshot);
    } catch (e: unknown) {
      if (options?.throwOnError) {
        throw e;
      }

      const msg = getErrorMessage(e);
      if (message?.error) {
        message.error(msg);
      } else {
        console.error(msg);
      }
    }
    set({ loading: false });
  },

  rebuildSnapshotFromDisk: async (options?: { throwOnError?: boolean }) => {
    set({ loading: true });
    try {
      const snapshot = await api.rebuildAppSnapshot();
      get().hydrateSnapshot(snapshot);
    } catch (e: unknown) {
      if (options?.throwOnError) {
        throw e;
      }

      const msg = getErrorMessage(e);
      if (message?.error) {
        message.error(msg);
      } else {
        console.error(msg);
      }
    }
    set({ loading: false });
  },

  startOperation: (key: string) => {
    set((state) => ({ operations: { ...state.operations, [key]: true } }));
  },

  finishOperation: (key: string) => {
    set((state) => {
      const next = { ...state.operations };
      delete next[key];
      return { operations: next };
    });
  },

  isOperationActive: (key: string) => {
    return get().operations[key] ?? false;
  },

  startDeploy: (instanceName: string, type: 'start' | 'upgrade' | 'downgrade') => {
    set({ deployState: { instanceName, deployType: type, progress: null } });
  },

  setDeployProgress: (progress: DeployProgress | null) => {
    set((state) => ({
      deployState: state.deployState ? { ...state.deployState, progress } : null,
    }));
  },

  closeDeploy: () => {
    set({ deployState: null });
  },
}));

// Event listener management (module-level, outside React)
let unlistenFns: UnlistenFn[] = [];
let listenersInitialized = false;

export async function initEventListeners() {
  if (listenersInitialized) return;
  listenersInitialized = true;

  const unlistenSnapshot = await listen<AppSnapshot>('app-snapshot', (event) => {
    useAppStore.getState().hydrateSnapshot(event.payload);
  });

  const unlistenDeploy = await listen<DeployProgress>('deploy-progress', (event) => {
    const progress = event.payload;
    const { deployState } = useAppStore.getState();

    if (deployState) {
      useAppStore.setState({
        deployState: { ...deployState, progress },
      });

      // Auto-close modal after done for all deploy types
      if (progress.step === 'done') {
        setTimeout(() => {
          useAppStore.setState({ deployState: null });
        }, MODAL_CLOSE_DELAY_MS);
      }
    }
  });

  unlistenFns = [unlistenSnapshot, unlistenDeploy];
}

export function cleanupEventListeners() {
  for (const fn of unlistenFns) {
    fn();
  }
  unlistenFns = [];
  listenersInitialized = false;
}
