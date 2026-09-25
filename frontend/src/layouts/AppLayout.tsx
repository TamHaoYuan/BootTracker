import { useEffect, useRef, useState } from 'react';
import { Layout, Menu, Button, Space, Tooltip, Badge, Drawer } from 'antd';
import type { MenuProps } from 'antd';
import {
  DashboardOutlined,
  FileTextOutlined,
  BarChartOutlined,
  SettingOutlined,
  ToolOutlined,
  MenuFoldOutlined,
  MenuUnfoldOutlined,
  MenuOutlined,
} from '@ant-design/icons';
import { Outlet, useNavigate, useLocation } from 'react-router-dom';
import { useThemeStore } from '../stores/themeStore';
import { useUiStore } from '../stores/uiStore';
import { useTheme } from '../hooks/useTheme';
import { useSettingsStore } from '../stores/settingsStore';
import { dataApi } from '../api/data';
import { versionApi } from '../api/version';

const { Sider, Header, Content } = Layout;

const SIDER_WIDTH = 208;
const SIDER_COLLAPSED_WIDTH = 64; // 折叠态优化：80→64px
const HEADER_HEIGHT = 56;
const MOBILE_BREAKPOINT = 768;

// 菜单分组：主功能组 + 系统组
const menuItems: MenuProps['items'] = [
  {
    type: 'group',
    label: '主功能',
    children: [
      { key: '/dashboard', icon: <DashboardOutlined />, label: '首页' },
      { key: '/records', icon: <FileTextOutlined />, label: '开机记录' },
      { key: '/charts', icon: <BarChartOutlined />, label: '图表' },
    ],
  },
  {
    type: 'group',
    label: '系统',
    children: [
      { key: '/admin', icon: <ToolOutlined />, label: '管理' },
      { key: '/settings', icon: <SettingOutlined />, label: '设置' },
    ],
  },
];

// Alt+数字快捷键映射
const ALT_KEY_MAP: Record<string, string> = {
  '1': '/dashboard',
  '2': '/records',
  '3': '/charts',
  '4': '/admin',
  '5': '/settings',
};

/* 日月图标：深色模式显示太阳（点击切浅色），浅色模式显示月亮（点击切深色） */
const SunIcon = ({ style }: { style?: React.CSSProperties }) => (
  <svg
    width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor"
    strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" style={style}
  >
    <circle cx="12" cy="12" r="5" />
    <line x1="12" y1="1" x2="12" y2="3" />
    <line x1="12" y1="21" x2="12" y2="23" />
    <line x1="4.22" y1="4.22" x2="5.64" y2="5.64" />
    <line x1="18.36" y1="18.36" x2="19.78" y2="19.78" />
    <line x1="1" y1="12" x2="3" y2="12" />
    <line x1="21" y1="12" x2="23" y2="12" />
    <line x1="4.22" y1="19.78" x2="5.64" y2="18.36" />
    <line x1="18.36" y1="5.64" x2="19.78" y2="4.22" />
  </svg>
);

const MoonIcon = ({ style }: { style?: React.CSSProperties }) => (
  <svg
    width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor"
    strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" style={style}
  >
    <path d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z" />
  </svg>
);

function formatNavTime(d: Date): string {
  const h = String(d.getHours()).padStart(2, '0');
  const m = String(d.getMinutes()).padStart(2, '0');
  const s = String(d.getSeconds()).padStart(2, '0');
  return `${h}:${m}:${s}`;
}

function AppLayout() {
  const navigate = useNavigate();
  const location = useLocation();
  const { mode } = useThemeStore();
  const { toggleMode } = useTheme();
  const { settings } = useSettingsStore();

  // 桌面：折叠状态跨会话记住（uiStore）+ 悬浮展开
  const collapsed = useUiStore((s) => s.sidebarCollapsed);
  const setCollapsed = useUiStore((s) => s.setSidebarCollapsed);
  const [hoverExpanded, setHoverExpanded] = useState(false);
  const effectiveCollapsed = collapsed && !hoverExpanded;

  // 移动：抽屉
  const [isMobile, setIsMobile] = useState(false);
  const [drawerOpen, setDrawerOpen] = useState(false);

  const [online, setOnline] = useState(false);
  const [navTime, setNavTime] = useState(() => formatNavTime(new Date()));
  const [appVersion, setAppVersion] = useState<string>('');

  // 隐藏 Admin 入口：点击版本号三次（1.5s 内）触发跳转
  const adminClickCountRef = useRef(0);
  const adminClickTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const handleVersionClick = () => {
    adminClickCountRef.current += 1;
    if (adminClickTimerRef.current) clearTimeout(adminClickTimerRef.current);
    adminClickTimerRef.current = setTimeout(() => {
      adminClickCountRef.current = 0;
    }, 1500);
    if (adminClickCountRef.current >= 3) {
      adminClickCountRef.current = 0;
      if (adminClickTimerRef.current) {
        clearTimeout(adminClickTimerRef.current);
        adminClickTimerRef.current = null;
      }
      navigate('/admin');
    }
  };

  useEffect(() => {
    void (async () => {
      try {
        const res = await dataApi.ping();
        setOnline(!!res?.ok);
      } catch {
        setOnline(false);
      }
    })();
  }, []);

  useEffect(() => {
    void (async () => {
      try {
        const info = await versionApi.get();
        if (info?.version) setAppVersion(info.version);
      } catch {
        // 后端不可用时静默
      }
    })();
  }, []);

  // 响应式：监听窗口宽度，<768px 切换为抽屉模式
  useEffect(() => {
    const mq = window.matchMedia(`(max-width: ${MOBILE_BREAKPOINT - 1}px)`);
    const onChange = (e: MediaQueryListEvent) => setIsMobile(e.matches);
    setIsMobile(mq.matches);
    mq.addEventListener('change', onChange);
    return () => mq.removeEventListener('change', onChange);
  }, []);

  // 键盘导航：Ctrl+Shift+A 进 Admin；Alt+1~5 切换菜单
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.ctrlKey && e.shiftKey && (e.key === 'A' || e.key === 'a')) {
        e.preventDefault();
        navigate('/admin');
        return;
      }
      if (e.altKey && !e.ctrlKey && !e.shiftKey && ALT_KEY_MAP[e.key]) {
        e.preventDefault();
        navigate(ALT_KEY_MAP[e.key]);
      }
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [navigate]);

  useEffect(() => {
    const t = setInterval(() => setNavTime(formatNavTime(new Date())), 1000);
    return () => clearInterval(t);
  }, []);

  const selectedKeys = [location.pathname === '/' ? '/dashboard' : location.pathname];

  const handleMenuClick: MenuProps['onClick'] = ({ key }) => {
    navigate(key);
    if (isMobile) setDrawerOpen(false);
  };

  const pageTitleMap: Record<string, string> = {
    '/dashboard': '开机记录',
    '/records': '开机记录',
    '/charts': '图表',
    '/admin': '管理',
    '/settings': '设置',
  };
  const pageTitle = pageTitleMap[selectedKeys[0]] ?? '开机记录';

  const hasCustomBg = !!settings?.customBgImage;

  // 装饰球仅在首页显示，减少其他页面视觉干扰
  const showOrbs = !hasCustomBg && selectedKeys[0] === '/dashboard';

  // 侧栏内层：品牌区 + 菜单 + 底部信息区
  const siderInner = (
    <div
      style={{
        display: 'flex',
        flexDirection: 'column',
        height: '100%',
        position: isMobile ? 'relative' : 'sticky',
        top: 0,
      }}
    >
      {/* 品牌区：展开时 图标+BootTracker 名称（与启动动画视觉连续） */}
      <div
        style={{
          height: HEADER_HEIGHT,
          display: 'flex',
          alignItems: 'center',
          gap: 10,
          padding: effectiveCollapsed && !isMobile ? '0' : '0 16px',
          justifyContent: effectiveCollapsed && !isMobile ? 'center' : 'flex-start',
          borderBottom: '1px solid var(--border-color)',
          overflow: 'hidden',
        }}
      >
        <img
          src="/icons/icon.png"
          alt="BootTracker"
          width={28}
          height={28}
          style={{ flexShrink: 0 }}
        />
        {(!effectiveCollapsed || isMobile) && (
          <span
            className="sider-brand-name"
            style={{
              fontSize: 16,
              fontWeight: 700,
              color: 'var(--accent)',
              letterSpacing: 0.3,
              whiteSpace: 'nowrap',
            }}
          >
            BootTracker
          </span>
        )}
      </div>

      <Menu
        mode="inline"
        theme={mode === 'dark' ? 'dark' : 'light'}
        selectedKeys={selectedKeys}
        items={menuItems}
        onClick={handleMenuClick}
        style={{
          flex: 1,
          borderRight: 'none',
          background: 'transparent',
          marginTop: 8,
          overflowY: 'auto',
          overflowX: 'hidden',
        }}
      />

      {/* 侧栏底部信息区：在线状态 + 时间 + 版本号 + 模式切换 */}
      <div
        style={{
          padding: (effectiveCollapsed && !isMobile) ? '8px 0' : '10px 12px',
          borderTop: '1px solid var(--border-color)',
          display: 'flex',
          flexDirection: (effectiveCollapsed && !isMobile) ? 'column' : 'row',
          alignItems: 'center',
          gap: 6,
          justifyContent: 'space-between',
        }}
      >
        {/* 折叠态：仅模式切换图标 */}
        {(effectiveCollapsed && !isMobile) ? (
          <>
            <Tooltip
              title={mode === 'dark' ? '切换到浅色模式' : '切换到深色模式'}
              placement="right"
            >
              <Button
                type="text"
                icon={mode === 'dark' ? <SunIcon /> : <MoonIcon />}
                onClick={toggleMode}
                block
                style={{ color: mode === 'dark' ? 'var(--accent)' : 'var(--text-secondary)' }}
              />
            </Tooltip>
            <Tooltip title={online ? '后端在线' : '后端离线'} placement="right">
              <Badge status={online ? 'success' : 'error'} />
            </Tooltip>
          </>
        ) : (
          /* 展开态：时间 + 版本号 + 在线 + 模式 */
          <>
            <div style={{ display: 'flex', flexDirection: 'column', gap: 4, minWidth: 0, flex: 1 }}>
              <div
                style={{
                  display: 'flex',
                  alignItems: 'center',
                  justifyContent: 'space-between',
                  gap: 8,
                }}
              >
                <Tooltip title={online ? '后端在线' : '后端离线'}>
                  <Badge
                    status={online ? 'success' : 'error'}
                    text={
                      <span style={{ color: 'var(--text-muted)', fontSize: 11 }}>
                        {online ? '在线' : '离线'}
                      </span>
                    }
                  />
                </Tooltip>
                <span
                  style={{
                    color: 'var(--text-secondary)',
                    fontSize: 11,
                    fontFamily: "'JetBrains Mono', monospace",
                    letterSpacing: 0.3,
                  }}
                >
                  {navTime}
                </span>
              </div>
              <div
                style={{
                  display: 'flex',
                  alignItems: 'center',
                  justifyContent: 'space-between',
                  gap: 8,
                }}
              >
                {appVersion && (
                  <Tooltip title="管理" placement="top">
                    <span
                      onClick={handleVersionClick}
                      style={{
                        color: 'var(--text-disabled)',
                        fontSize: 10,
                        cursor: 'pointer',
                        userSelect: 'none',
                      }}
                    >
                      v{appVersion}
                    </span>
                  </Tooltip>
                )}
                <Tooltip
                  title={mode === 'dark' ? '切换到浅色模式' : '切换到深色模式'}
                  placement="top"
                >
                  <Button
                    type="text"
                    size="small"
                    icon={mode === 'dark' ? <SunIcon style={{ width: 14, height: 14 }} /> : <MoonIcon style={{ width: 14, height: 14 }} />}
                    onClick={toggleMode}
                    style={{ color: mode === 'dark' ? 'var(--accent)' : 'var(--text-secondary)' }}
                  />
                </Tooltip>
              </div>
            </div>
          </>
        )}
      </div>
    </div>
  );

  return (
    <div
      style={{
        position: 'relative',
        minHeight: '100vh',
        width: '100%',
        background: hasCustomBg
          ? `url(${settings!.customBgImage}) center/cover no-repeat`
          : 'var(--bg-page)',
        transition: 'background 0.4s ease',
      }}
    >
      {/* Glass Orb 背景装饰球 — 仅首页显示，继承旧版 Dark Gallery 风格 */}
      {showOrbs && (
        <>
          <div className="glass-orb orb-1" />
          <div className="glass-orb orb-2" />
          <div className="glass-orb orb-3" />
        </>
      )}
      {hasCustomBg && (
        <div
          style={{
            position: 'absolute',
            inset: 0,
            background: mode === 'dark'
              ? 'rgba(10, 10, 12, 0.72)'
              : 'rgba(248, 250, 252, 0.72)',
            backdropFilter: 'blur(2px)',
            zIndex: 0,
            pointerEvents: 'none',
          }}
        />
      )}

      <Layout style={{ minHeight: '100vh', background: 'transparent', position: 'relative', zIndex: 1 }}>
        {/* 桌面：悬浮 Sider；移动：抽屉由顶栏触发 */}
        {!isMobile && (
          <Sider
            width={SIDER_WIDTH}
            collapsedWidth={SIDER_COLLAPSED_WIDTH}
            collapsed={effectiveCollapsed}
            onCollapse={setCollapsed}
            collapsible
            trigger={null}
            onMouseEnter={() => { if (collapsed) setHoverExpanded(true); }}
            onMouseLeave={() => setHoverExpanded(false)}
            style={{
              background: 'var(--bg-card)',
              borderRight: '1px solid var(--border-color)',
              // fixed：Sider 固定视口左侧，滚动时始终覆盖左侧背景。
              // 原用 absolute + height:100vh，内容超一屏后 Sider 上移消失，
              // 左侧露出外层 --bg-page 与 Sider 的 --bg-card 色差 → 下滑背景断层。
              position: 'fixed',
              left: 0,
              top: 0,
              zIndex: 100,
              height: '100vh',
              boxShadow: effectiveCollapsed ? 'none' : '4px 0 24px rgba(0,0,0,0.15)',
            }}
          >
            {siderInner}
          </Sider>
        )}
        {isMobile && (
          <Drawer
            placement="left"
            open={drawerOpen}
            onClose={() => setDrawerOpen(false)}
            width={SIDER_WIDTH}
            styles={{
              body: {
                padding: 0,
                background: 'var(--bg-card)',
                borderRight: '1px solid var(--border-color)',
              },
              header: { display: 'none' },
            }}
          >
            {siderInner}
          </Drawer>
        )}

        <Layout
          style={{
            background: 'transparent',
            marginLeft: isMobile ? 0 : (collapsed ? SIDER_COLLAPSED_WIDTH : SIDER_WIDTH),
            transition: 'margin-left 0.2s var(--ease-out-expo)',
          }}
        >
          <Header
            style={{
              height: HEADER_HEIGHT,
              padding: '0 20px',
              background: 'var(--bg-card)',
              borderBottom: '1px solid var(--border-color)',
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'space-between',
              position: 'sticky',
              top: 0,
              zIndex: 10,
            }}
          >
            <Space size="middle" align="center">
              {isMobile ? (
                <Tooltip title="打开菜单">
                  <Button
                    type="text"
                    icon={<MenuOutlined />}
                    onClick={() => setDrawerOpen(true)}
                    style={{ fontSize: 16, color: 'var(--text-secondary)' }}
                  />
                </Tooltip>
              ) : (
                <Tooltip title={collapsed ? '展开侧栏' : '收起侧栏'}>
                  <Button
                    type="text"
                    icon={collapsed ? <MenuUnfoldOutlined /> : <MenuFoldOutlined />}
                    onClick={() => setCollapsed(!collapsed)}
                    style={{ fontSize: 16, color: 'var(--text-secondary)' }}
                  />
                </Tooltip>
              )}
              <span
                style={{
                  fontSize: 15,
                  fontWeight: 600,
                  color: 'var(--text-primary)',
                }}
              >
                {pageTitle}
              </span>
            </Space>
          </Header>
          <Content
            className="app-content"
            style={{
              minHeight: `calc(100vh - ${HEADER_HEIGHT}px)`,
              padding: 'var(--content-pad, 24px)',
              background: 'var(--bg-content)',
            }}
          >
            {/* key=pathname：路由切换时重挂载，重放非线性入场动画 */}
            <div key={location.pathname} className="page-enter">
              <Outlet />
            </div>
          </Content>
        </Layout>
      </Layout>
    </div>
  );
}

export default AppLayout;
