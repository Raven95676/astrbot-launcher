import { useCallback } from 'react';
import { message } from '../antdStatic';
import { api } from '../api';
import type { InstanceStatus } from '../types';
import { handleApiError } from '../utils';
import { STATUS_MESSAGES, OPERATION_KEYS } from '../constants';
import { useAppStore } from '../stores';

/**
 * Hook for handling instance version upgrade flow.
 * The backend now handles the full pipeline: backup → deploy → restore → cleanup.
 */
export function useInstanceUpgrade() {
  const startDeploy = useAppStore((s) => s.startDeploy);
  const closeDeploy = useAppStore((s) => s.closeDeploy);
  const startOperation = useAppStore((s) => s.startOperation);
  const finishOperation = useAppStore((s) => s.finishOperation);
  const reloadSnapshot = useAppStore((s) => s.reloadSnapshot);

  const upgradeInstance = useCallback(
    async (instance: InstanceStatus, newName: string, newVersion: string): Promise<boolean> => {
      try {
        await reloadSnapshot();
        const { instances } = useAppStore.getState();
        const latestInstance = instances.find((i) => i.id === instance.id);
        if (!latestInstance) {
          message.warning('实例不存在或已被删除');
          closeDeploy();
          return false;
        }

        const cmp = await api.compareVersions(newVersion, instance.version);
        const deployType = cmp > 0 ? 'upgrade' : 'downgrade';
        startDeploy(latestInstance.name, deployType);
        startOperation(OPERATION_KEYS.instance(instance.id));

        await api.updateInstance(instance.id, newName, newVersion);
        await reloadSnapshot();
        message.success(STATUS_MESSAGES.INSTANCE_UPDATED);
        // done event from backend auto-closes the modal via event listener
        return true;
      } catch (error) {
        handleApiError(error);
        closeDeploy();
        return false;
      } finally {
        finishOperation(OPERATION_KEYS.instance(instance.id));
      }
    },
    [startDeploy, closeDeploy, startOperation, finishOperation, reloadSnapshot]
  );

  return { upgradeInstance };
}
