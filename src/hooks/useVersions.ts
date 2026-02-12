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
  handleInstallPython: () => Promise<void>;
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
        const { pythonInstalled, versions } = useAppStore.getState();
        if (!pythonInstalled) {
          message.warning('请先安装 Python 运行时');
          return;
        }
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

  const handleInstallPython = useCallback(async () => {
    const key = OPERATION_KEYS.installPython;
    startOperation(key);
    try {
      await reloadSnapshot();
      const { pythonInstalled } = useAppStore.getState();
      if (pythonInstalled) {
        message.info('Python 运行时已安装');
        return;
      }

      const result = await api.installPython();
      await reloadSnapshot();
      const { pythonInstalled: installedAfter } = useAppStore.getState();
      if (!installedAfter) {
        message.error(`Python 安装未生效：${result}`);
        return;
      }

      message.success(result);
    } catch (error) {
      handleApiError(error);
    } finally {
      finishOperation(key);
    }
  }, [startOperation, finishOperation, reloadSnapshot]);

  return {
    handleInstall,
    handleUninstall,
    handleInstallPython,
  };
}
