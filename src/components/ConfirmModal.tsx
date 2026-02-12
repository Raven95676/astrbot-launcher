import { Modal, Space } from 'antd';
import { ExclamationCircleOutlined, WarningOutlined } from '@ant-design/icons';

interface ConfirmModalProps {
  open: boolean;
  title: string;
  content: React.ReactNode;
  loading?: boolean;
  danger?: boolean;
  okText?: string;
  onConfirm: () => void;
  onCancel: () => void;
}

export function ConfirmModal({
  open,
  title,
  content,
  loading = false,
  danger = false,
  okText = '确定',
  onConfirm,
  onCancel,
}: ConfirmModalProps) {
  return (
    <Modal
      title={
        <Space>
          {danger ? (
            <WarningOutlined style={{ color: '#ff4d4f' }} />
          ) : (
            <ExclamationCircleOutlined style={{ color: '#faad14' }} />
          )}
          {title}
        </Space>
      }
      open={open}
      onOk={onConfirm}
      onCancel={onCancel}
      okText={okText}
      cancelText="取消"
      okButtonProps={{ danger, loading }}
      cancelButtonProps={{ disabled: loading }}
      closable={false}
    >
      {content}
    </Modal>
  );
}

interface ConfirmDeleteModalProps {
  open: boolean;
  title: string;
  itemName: string;
  loading?: boolean;
  onConfirm: () => void;
  onCancel: () => void;
}

export function ConfirmDeleteModal({
  open,
  title,
  itemName,
  loading = false,
  onConfirm,
  onCancel,
}: ConfirmDeleteModalProps) {
  return (
    <ConfirmModal
      open={open}
      title={title}
      danger
      loading={loading}
      okText="确认删除"
      onConfirm={onConfirm}
      onCancel={onCancel}
      content={
        <p>
          确定要删除 <strong>{itemName}</strong> 吗？此操作不可恢复。
        </p>
      }
    />
  );
}
