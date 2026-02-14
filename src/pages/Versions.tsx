import { useState, useEffect, useCallback } from 'react';
import { Button, Card, Space, Tag, Typography, List, Drawer, Alert, Tooltip } from 'antd';
import {
  DownloadOutlined,
  DeleteOutlined,
  ReloadOutlined,
  InfoCircleOutlined,
} from '@ant-design/icons';
import type { InstalledVersion, GitHubRelease } from '../api';
import { useReleases } from '../hooks';
import { useVersions } from '../hooks/useVersions';
import { useAppStore } from '../stores';
import { ConfirmModal } from '../components';
import { OPERATION_KEYS } from '../constants';

const { Title, Text, Paragraph } = Typography;

export default function Versions() {
  const versions = useAppStore((s) => s.versions);
  const pythonInstalled = useAppStore((s) => s.pythonInstalled);
  const config = useAppStore((s) => s.config);
  const appLoading = useAppStore((s) => s.loading);
  const rebuildSnapshotFromDisk = useAppStore((s) => s.rebuildSnapshotFromDisk);
  const operations = useAppStore((s) => s.operations);

  const [detailRelease, setDetailRelease] = useState<GitHubRelease | null>(null);
  const [detailOpen, setDetailOpen] = useState(false);
  const [uninstallOpen, setUninstallOpen] = useState(false);
  const [versionToUninstall, setVersionToUninstall] = useState<InstalledVersion | null>(null);

  const { releases, loading: releasesLoading, fetchReleases } = useReleases();

  const { handleInstall, handleUninstall: doUninstall, handleInstallPython } = useVersions();

  const refreshAll = useCallback(
    async (forceRefresh = false) => {
      await Promise.all([rebuildSnapshotFromDisk(), fetchReleases(forceRefresh)]);
    },
    [rebuildSnapshotFromDisk, fetchReleases]
  );

  useEffect(() => {
    fetchReleases();
  }, [fetchReleases]);

  const handleUninstall = async () => {
    if (!versionToUninstall) return;
    await doUninstall(versionToUninstall);
    setUninstallOpen(false);
    setVersionToUninstall(null);
  };

  const isInstalled = (tagName: string) => versions.some((v) => v.version === tagName);
  const availableReleases = releases.filter((r) => !isInstalled(r.tag_name));
  const getInstalledRelease = (version: string) => releases.find((r) => r.tag_name === version);

  return (
    <>
      <div
        style={{
          display: 'flex',
          justifyContent: 'space-between',
          alignItems: 'center',
          marginBottom: 16,
        }}
      >
        <Title level={4} style={{ margin: 0 }}>
          版本管理
        </Title>
        <Button
          icon={<ReloadOutlined />}
          onClick={() => refreshAll(true)}
          loading={appLoading || releasesLoading}
        >
          刷新
        </Button>
      </div>

      {/* Python Status */}
      {config &&
        (pythonInstalled ? (
          <Alert title="Python 运行时已就绪" type="success" showIcon style={{ marginBottom: 16 }} />
        ) : (
          <Alert
            title="需要安装 Python 运行时以启动 AstrBot 实例"
            type="warning"
            showIcon
            style={{ marginBottom: 16 }}
            action={
              <Button
                type="primary"
                size="small"
                icon={<DownloadOutlined />}
                loading={operations[OPERATION_KEYS.installPython] || false}
                onClick={handleInstallPython}
              >
                安装 Python
              </Button>
            }
          />
        ))}

      {/* Installed Versions */}
      <Card title="已下载的版本" size="small" style={{ marginBottom: 16 }}>
        <List
          dataSource={versions}
          locale={{ emptyText: '暂无已下载的版本' }}
          renderItem={(item) => {
            const release = getInstalledRelease(item.version);
            return (
              <List.Item
                actions={[
                  release && (
                    <Tooltip title="详情" key="detail">
                      <Button
                        type="text"
                        icon={<InfoCircleOutlined />}
                        onClick={() => {
                          setDetailRelease(release);
                          setDetailOpen(true);
                        }}
                      />
                    </Tooltip>
                  ),
                  <Tooltip title="卸载" key="uninstall">
                    <Button
                      type="text"
                      danger
                      icon={<DeleteOutlined />}
                      disabled={uninstallOpen && versionToUninstall?.version === item.version}
                      onClick={() => {
                        setVersionToUninstall(item);
                        setUninstallOpen(true);
                      }}
                    />
                  </Tooltip>,
                ].filter(Boolean)}
              >
                <List.Item.Meta
                  title={
                    <Space>
                      {release?.name || item.version}
                      {release?.prerelease && <Tag color="orange">预发行</Tag>}
                    </Space>
                  }
                  description={release ? new Date(release.published_at).toLocaleDateString() : null}
                />
              </List.Item>
            );
          }}
        />
      </Card>

      {/* Available Versions */}
      <Card title="可下载的版本" size="small">
        <List
          dataSource={availableReleases}
          loading={releasesLoading && releases.length === 0}
          locale={{
            emptyText: releases.length === 0 ? '加载中...' : '所有版本均已下载',
          }}
          renderItem={(release) => {
            const key = release.tag_name;
            return (
              <List.Item
                actions={[
                  <Tooltip title="详情" key="detail">
                    <Button
                      type="text"
                      icon={<InfoCircleOutlined />}
                      onClick={() => {
                        setDetailRelease(release);
                        setDetailOpen(true);
                      }}
                    />
                  </Tooltip>,
                  <Tooltip title={pythonInstalled ? '下载' : '请先安装 Python'} key="install">
                    <Button
                      type="text"
                      icon={<DownloadOutlined />}
                      loading={operations[OPERATION_KEYS.installVersion(key)]}
                      disabled={!pythonInstalled}
                      onClick={() => handleInstall(release)}
                    />
                  </Tooltip>,
                ]}
              >
                <List.Item.Meta
                  title={
                    <Space>
                      {release.name || release.tag_name}
                      {release.prerelease && <Tag color="orange">预发行</Tag>}
                    </Space>
                  }
                  description={new Date(release.published_at).toLocaleDateString()}
                />
              </List.Item>
            );
          }}
        />
      </Card>

      {/* Release Detail Drawer */}
      <Drawer
        title={detailRelease?.name || detailRelease?.tag_name || '版本详情'}
        open={detailOpen}
        onClose={() => setDetailOpen(false)}
        size={500}
      >
        {detailRelease && (
          <Space orientation="vertical" style={{ width: '100%' }}>
            <div>
              <Text strong>版本: </Text>
              <Text>{detailRelease.tag_name}</Text>
            </div>
            <div>
              <Text strong>发布时间: </Text>
              <Text>{new Date(detailRelease.published_at).toLocaleString()}</Text>
            </div>
            {detailRelease.prerelease && <Tag color="orange">预发行版本</Tag>}
            <div style={{ marginTop: 16 }}>
              <Text strong>发布说明:</Text>
              <div
                style={{
                  marginTop: 8,
                  padding: 12,
                  background: '#f5f5f5',
                  borderRadius: 8,
                  maxHeight: 400,
                  overflow: 'auto',
                }}
              >
                {detailRelease.body ? (
                  <Paragraph style={{ whiteSpace: 'pre-wrap', margin: 0 }}>
                    {detailRelease.body}
                  </Paragraph>
                ) : (
                  <Text type="secondary">无发布说明</Text>
                )}
              </div>
            </div>
            <div style={{ marginTop: 16 }}>
              <Button
                type="link"
                href={detailRelease.html_url}
                target="_blank"
                style={{ padding: 0 }}
              >
                在 GitHub 上查看
              </Button>
            </div>
          </Space>
        )}
      </Drawer>

      {/* Uninstall Modal */}
      <ConfirmModal
        open={uninstallOpen}
        title="确认卸载"
        danger
        content={
          <>
            <p>确定卸载此版本？</p>
            {versionToUninstall && (
              <p style={{ color: '#666' }}>版本: {versionToUninstall.version}</p>
            )}
          </>
        }
        loading={
          versionToUninstall
            ? operations[OPERATION_KEYS.uninstallVersion(versionToUninstall.version)] || false
            : false
        }
        onConfirm={handleUninstall}
        onCancel={() => {
          setUninstallOpen(false);
          setVersionToUninstall(null);
        }}
      />
    </>
  );
}
