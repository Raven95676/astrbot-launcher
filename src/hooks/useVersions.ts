import { useCallback } from 'react';
import { message } from '../antdStatic';
import { api } from '../api';
import type { InstalledVersion, GitHubRelease } from '../types';
import { handleApiError } from '../utils';
import { OPERATION_KEYS } from '../constants';
import { useAppStore } from '../stores';

interface UseVersionsReturn {
  handleInstall: (release: GitHubRelease) => Promise<void>;
  handleUninstall: (version: InstalledVersion) => Promise<void>;
}

export function useVersions(): UseVersionsReturn {
  const startOperation = useAppStore((s) => s.startOperation);
  const finishOperation = useAppStore((s) => s.finishOperation);
  const reloadSnapshot = useAppStore((s) => s.reloadSnapshot);

  const handleInstall = useCallback(
    async (release: GitHubRelease) => {
      const key = OPERATION_KEYS.installVersion(release.tag_name);
      startOperation(key);
      try {
        await reloadSnapshot();
        const { versions } = useAppStore.getState();
        if (versions.some((v) => v.version === release.tag_name)) {
          message.info(`版本 ${release.tag_name} 已下载`);
          return;
        }

        await api.installVersion(release);
        await reloadSnapshot({ throwOnError: true });
        message.success(`版本 ${release.tag_name} 下载成功`);
      } catch (error) {
        handleApiError(error);
      } finally {
        finishOperation(key);
      }
    },
    [startOperation, finishOperation, reloadSnapshot]
  );

  const handleUninstall = useCallback(
    async (version: InstalledVersion) => {
      const key = OPERATION_KEYS.uninstallVersion(version.version);
      startOperation(key);
      try {
        await reloadSnapshot();
        const { versions } = useAppStore.getState();
        if (!versions.some((v) => v.version === version.version)) {
          message.info(`版本 ${version.version} 已卸载`);
          return;
        }

        await api.uninstallVersion(version.version);
        await reloadSnapshot({ throwOnError: true });
        message.success('已卸载');
      } catch (error) {
        handleApiError(error);
      } finally {
        finishOperation(key);
      }
    },
    [startOperation, finishOperation, reloadSnapshot]
  );

  return {
    handleInstall,
    handleUninstall,
  };
}
