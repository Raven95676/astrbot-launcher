import { BrowserRouter, Routes, Route, useNavigate, useLocation } from 'react-router-dom';
import { useEffect } from 'react';
import { Layout, Menu, ConfigProvider, App as AntdApp, theme } from 'antd';
import zhCN from 'antd/locale/zh_CN';
import {
  DesktopOutlined,
  CloudDownloadOutlined,
  SaveOutlined,
  ToolOutlined,
} from '@ant-design/icons';
import { ErrorBoundary } from './components';
import { AntdStaticProvider } from './antdStatic';
import { useAppStore, initEventListeners, cleanupEventListeners } from './stores';
import Dashboard from './pages/Dashboard';
import Versions from './pages/Versions';
import Backup from './pages/Backup';
import Advanced from './pages/Advanced';
import WebUIView from './pages/WebUIView';
import './App.css';

const { Sider, Content } = Layout;

function AppLayout() {
  const navigate = useNavigate();
  const location = useLocation();
  const reloadSnapshot = useAppStore((s) => s.reloadSnapshot);

  useEffect(() => {
    void reloadSnapshot();
  }, [location.pathname, reloadSnapshot]);

  const menuItems = [
    {
      key: '/',
      icon: <DesktopOutlined />,
      label: '实例',
    },
    {
      key: '/versions',
      icon: <CloudDownloadOutlined />,
      label: '版本',
    },
    {
      key: '/backup',
      icon: <SaveOutlined />,
      label: '备份',
    },
    {
      key: '/advanced',
      icon: <ToolOutlined />,
      label: '高级',
    },
  ];

  return (
    <Layout style={{ minHeight: '100vh' }}>
      <Sider
        width={180}
        theme="light"
        style={{
          overflow: 'auto',
          height: '100vh',
          position: 'fixed',
          left: 0,
          top: 0,
          bottom: 0,
        }}
      >
        <div
          style={{
            height: 48,
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            fontWeight: 700,
            fontSize: 16,
            borderBottom: '1px solid #f0f0f0',
          }}
        >
          AstrBot Launcher
        </div>
        <Menu
          mode="inline"
          selectedKeys={[location.pathname]}
          items={menuItems}
          onClick={({ key }) => navigate(key)}
          style={{ borderRight: 0 }}
        />
      </Sider>
      <Layout style={{ marginLeft: 180 }}>
        <Content style={{ padding: 24, overflow: 'auto', height: '100vh' }}>
          <ErrorBoundary>
            <Routes>
              <Route path="/" element={<Dashboard />} />
              <Route path="/versions" element={<Versions />} />
              <Route path="/backup" element={<Backup />} />
              <Route path="/advanced" element={<Advanced />} />
            </Routes>
          </ErrorBoundary>
        </Content>
      </Layout>
    </Layout>
  );
}

function App() {
  useEffect(() => {
    void initEventListeners();
    void useAppStore.getState().reloadSnapshot();

    return () => {
      cleanupEventListeners();
    };
  }, []);

  return (
    <ConfigProvider
      locale={zhCN}
      theme={{
        algorithm: theme.defaultAlgorithm,
        token: {
          borderRadius: 8,
        },
      }}
    >
      <AntdApp>
        <AntdStaticProvider />
        <BrowserRouter>
          <Routes>
            <Route path="/webui/:instanceId" element={<WebUIView />} />
            <Route path="/*" element={<AppLayout />} />
          </Routes>
        </BrowserRouter>
      </AntdApp>
    </ConfigProvider>
  );
}

export default App;
