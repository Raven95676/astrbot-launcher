import { Button, Space, Tooltip } from 'antd';
import {
  PlayCircleOutlined,
  PauseCircleOutlined,
  ReloadOutlined,
  DeleteOutlined,
  GlobalOutlined,
  SettingOutlined,
} from '@ant-design/icons';
import type { InstanceStatus } from '../types';

interface InstanceActionsProps {
  instance: InstanceStatus;
  pythonInstalled: boolean;
  loading: boolean;
  isDeploying: boolean;
  isDeleting: boolean;
  onStart: (id: string) => void;
  onStop: (id: string) => void;
  onRestart: (id: string) => void;
  onOpen: (instance: InstanceStatus) => void;
  onEdit: (instance: InstanceStatus) => void;
  onDelete: (instance: InstanceStatus) => void;
}

export function InstanceActions({
  instance,
  pythonInstalled,
  loading,
  isDeploying,
  isDeleting,
  onStart,
  onStop,
  onRestart,
  onOpen,
  onEdit,
  onDelete,
}: InstanceActionsProps) {
  return (
    <Space size="small">
      {instance.running ? (
        <>
          <Tooltip title="停止">
            <Button
              type="text"
              icon={<PauseCircleOutlined />}
              loading={loading}
              onClick={() => onStop(instance.id)}
            />
          </Tooltip>
          <Tooltip title="重启">
            <Button
              type="text"
              icon={<ReloadOutlined />}
              loading={loading}
              onClick={() => onRestart(instance.id)}
            />
          </Tooltip>
          <Tooltip title={instance.dashboard_enabled ? '打开 WebUI' : 'Dashboard 已禁用'}>
            <Button
              type="text"
              icon={<GlobalOutlined />}
              disabled={!instance.dashboard_enabled}
              onClick={() => onOpen(instance)}
            />
          </Tooltip>
        </>
      ) : (
        <Tooltip title={pythonInstalled ? '启动' : '请先在版本页面安装 Python'}>
          <Button
            type="text"
            icon={<PlayCircleOutlined style={{ color: pythonInstalled ? '#52c41a' : undefined }} />}
            loading={loading || isDeploying}
            disabled={!pythonInstalled}
            onClick={() => onStart(instance.id)}
          />
        </Tooltip>
      )}
      <Tooltip title="设置">
        <Button
          type="text"
          icon={<SettingOutlined />}
          disabled={instance.running || isDeploying}
          onClick={() => onEdit(instance)}
        />
      </Tooltip>
      <Tooltip title={instance.running ? '请先停止实例' : '删除'}>
        <Button
          type="text"
          danger
          icon={<DeleteOutlined />}
          disabled={instance.running || isDeploying || isDeleting}
          onClick={() => onDelete(instance)}
        />
      </Tooltip>
    </Space>
  );
}
