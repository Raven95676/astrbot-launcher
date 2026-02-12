import { Tag, Space, Tooltip } from 'antd';
import { WarningOutlined } from '@ant-design/icons';
import type { InstanceStatus, DeployProgress } from '../types';

interface InstanceStatusTagProps {
  instance: InstanceStatus;
  deployProgress?: DeployProgress | null;
}

export function InstanceStatusTag({ instance, deployProgress }: InstanceStatusTagProps) {
  const isDeploying =
    deployProgress &&
    deployProgress.instance_id === instance.id &&
    deployProgress.step !== 'done' &&
    deployProgress.step !== 'error';

  if (isDeploying) {
    return <Tag color="processing">部署中</Tag>;
  }

  return (
    <Space size="small">
      <Tag color={instance.running ? 'green' : 'default'}>
        {instance.running ? '运行中' : '已停止'}
      </Tag>
      {instance.running && !instance.dashboard_enabled && (
        <Tooltip title="Launcher无法正确追踪此实例运行状态">
          <WarningOutlined style={{ color: '#faad14' }} />
        </Tooltip>
      )}
    </Space>
  );
}
