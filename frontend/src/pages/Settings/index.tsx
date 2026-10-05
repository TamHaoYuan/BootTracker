import { useEffect, useState, useCallback, useRef } from 'react';
import {
  Tabs,
  Card,
  Form,
  Segmented,
  Radio,
  Switch,
  InputNumber,
  Upload,
  Button,
  Space,
  App as AntdApp,
  Typography,
  Input,
  Timeline,
  Tag,
  Tooltip,
  Spin,
} from 'antd';
import type { UploadProps } from 'antd';
import {
  CheckCircleFilled,
  CopyOutlined,
  UploadOutlined,
  DeleteOutlined,
  ReloadOutlined,
  SyncOutlined,
  LogoutOutlined,
  CheckOutlined,
  CloudServerOutlined,
  InfoCircleOutlined,
  DownloadOutlined,
  RocketOutlined,
  ArrowUpOutlined,
} from '@ant-design/icons';
import {
  settingsApi,
  windowApi,
  systemApi,
  tunnelApi,
  versionApi,
} from '../../api';
import type { AppSettings, VersionHistoryEntry, CheckUpdateResponse } from '../../api/types';
import { useSettingsStore } from '../../stores/settingsStore';
import { useThemeStore, applyThemeToDom } from '../../stores/themeStore';
import { native, isNativeAvailable } from '../../bridge';
import { THEME_ACCENTS, type ThemeName } from '../../styles/antd-theme';

const { Title, Text, Link } = Typography;
const { Password } = Input;

const TAB_GAP = 16;

const CARD_STYLE: React.CSSProperties = {
  borderRadius: 12,
  background: 'var(--bg-card)',
  borderColor: 'var(--border-color)',
};

const CARD_BODY_STYLE: React.CSSProperties = {
  padding: 24,
};

const LABEL_STYLE: React.CSSProperties = {
  color: 'var(--text-secondary)',
  fontWeight: 500,
};

const AVAILABLE_THEMES: ThemeName[] = [
  'purple',
  'blue',
  'green',
  'orange',
  'gray',
  'mica',
  'material-you',
];

const THEME_LABELS: Record<ThemeName, string> = {
  purple: '紫色',
  blue: '蓝色',
  green: '绿色',
  orange: '橙色',
  gray: '灰色',
  mica: '云母',
  'material-you': 'Material You',
};

function Settings() {
  const { message, modal } = AntdApp.useApp();
  const { settings, setSettings, patch } = useSettingsStore();
  const { mode: storeMode, theme: storeTheme, setMode, setTheme } = useThemeStore();

  const [loading, setLoading] = useState(true);
  const [tunnelUrl, setTunnelUrl] = useState<string | null>(null);
  const [tunnelUrlLoading, setTunnelUrlLoading] = useState(false);
  const [versionInfo, setVersionInfo] = useState<{ version: string; appMode: string } | null>(null);
  const [versionHistory, setVersionHistory] = useState<VersionHistoryEntry[]>([]);
  const [historyLoading, setHistoryLoading] = useState(false);
  const [bumpNotes, setBumpNotes] = useState('');
  const [checkResult, setCheckResult] = useState<CheckUpdateResponse | null>(null);
  const [checkLoading, setCheckLoading] = useState(false);

  const rollbackRef = useRef<Record<string, unknown>>({});

  const loadSettings = useCallback(async () => {
    setLoading(true);
    try {
      const s = await settingsApi.get();
      setSettings(s);
    } catch (e) {
      message.error(`加载设置失败：${(e as Error).message}`);
    } finally {
      setLoading(false);
    }
  }, [setSettings, message]);

  const fetchTunnelUrl = useCallback(async () => {
    setTunnelUrlLoading(true);
    try {
      const r = await tunnelApi.getUrl();
      setTunnelUrl(r.url);
    } catch {
      setTunnelUrl(null);
    } finally {
      setTunnelUrlLoading(false);
    }
  }, []);

  const fetchVersionInfo = useCallback(async () => {
    try {
      const v = await versionApi.get();
      let appMode = '生产';
      try {
        const info = await native<{ mode?: string }>('getAppInfo');
        if (info?.mode) {
          appMode = info.mode === 'dev' ? '开发' : '生产';
        }
      } catch {
        if (location.protocol === 'http:' && (location.hostname === '127.0.0.1' || location.hostname === 'localhost')) {
          appMode = '开发';
        }
      }
      setVersionInfo({ version: v.version, appMode });
    } catch (e) {
      message.error(`加载版本信息失败：${(e as Error).message}`);
    }
  }, [message]);

  const fetchVersionHistory = useCallback(async () => {
    setHistoryLoading(true);
    try {
      const h = await versionApi.history();
      setVersionHistory(h.sort((a, b) => new Date(b.date).getTime() - new Date(a.date).getTime()));
    } catch (e) {
      message.error(`加载版本历史失败：${(e as Error).message}`);
    } finally {
      setHistoryLoading(false);
    }
  }, [message]);

  useEffect(() => {
    void loadSettings();
    void fetchVersionInfo();
    void fetchVersionHistory();
  }, [loadSettings, fetchVersionInfo, fetchVersionHistory]);

  useEffect(() => {
    if (settings?.tunnelEnabled) {
      void fetchTunnelUrl();
    } else {
      setTunnelUrl(null);
    }
  }, [settings?.tunnelEnabled, fetchTunnelUrl]);

  const updateField = useCallback(
    async <K extends keyof AppSettings>(key: K, value: AppSettings[K], rollbackValue?: AppSettings[K]) => {
      const prev = settings?.[key];
      patch({ [key]: value } as Partial<AppSettings>);
      rollbackRef.current[key as string] = rollbackValue !== undefined ? rollbackValue : prev;
      try {
        await settingsApi.update({ [key]: value } as Partial<AppSettings>);
        message.success('已保存');
      } catch (e) {
        const rb = rollbackRef.current[key as string] as AppSettings[K] | undefined;
        if (rb !== undefined) {
          patch({ [key]: rb } as Partial<AppSettings>);
        }
        message.error(`保存失败：${(e as Error).message}`);
      }
    },
    [settings, patch, message],
  );

  const handleModeChange = useCallback(
    async (newMode: 'dark' | 'light') => {
      const prevMode = storeMode;
      setMode(newMode);
      applyThemeToDom(newMode, storeTheme);
      try {
        // 本地 Tauri 窗口走 IPC（低延迟、不走网络）；
        // 远程浏览器 native 不可用，降级 HTTP 同步桌面端背景
        if (isNativeAvailable()) {
          await native('setTheme', { mode: newMode });
        } else {
          await windowApi.setTheme(newMode);
        }
        message.success('主题已切换');
      } catch (e) {
        setMode(prevMode);
        applyThemeToDom(prevMode, storeTheme);
        message.error(`切换主题失败：${(e as Error).message}`);
        return;
      }
      try {
        await settingsApi.update({ appMode: newMode });
      } catch (e) {
        message.warning(`主题设置未持久化：${(e as Error).message}`);
      }
    },
    [storeMode, storeTheme, setMode, message],
  );

  const handleThemeChange = useCallback(
    (newTheme: ThemeName) => {
      // 主题变体仅前端持久化（与旧版 app.js switchTheme 行为一致）
      setTheme(newTheme);
      applyThemeToDom(storeMode, newTheme);
      message.success('主题变体已切换');
    },
    [storeMode, storeTheme, setTheme, message],
  );

  const handleBgUpload: UploadProps['customRequest'] = async (options) => {
    const file = options.file as File;
    if (file.size > 10 * 1024 * 1024) {
      message.error('图片大小不能超过 10MB');
      options.onError?.(new Error('File too large'));
      return;
    }
    if (!file.type.startsWith('image/')) {
      message.error('请选择图片文件');
      options.onError?.(new Error('Invalid file type'));
      return;
    }
    const prevBg = settings?.customBgImage ?? '';
    try {
      const r = await systemApi.uploadBgImage(file);
      // 时间戳 bust 缓存：背景图 URL 固定（bg.jpg），不换 URL 会用旧缓存图
      const bgUrl = `${r.path}?t=${Date.now()}`;
      patch({ customBgImage: bgUrl });
      try {
        await settingsApi.update({ customBgImage: bgUrl });
        message.success('背景图上传成功');
        options.onSuccess?.(r);
      } catch (e) {
        patch({ customBgImage: prevBg });
        message.error(`保存背景图失败：${(e as Error).message}`);
        options.onError?.(e as Error);
      }
    } catch (e) {
      message.error(`上传背景图失败：${(e as Error).message}`);
      options.onError?.(e as Error);
    }
  };

  const handleClearBg = () => {
    const prevBg = settings?.customBgImage ?? '';
    patch({ customBgImage: '' });
    settingsApi
      .update({ customBgImage: '' })
      .then(() => message.success('已清除自定义背景'))
      .catch((e) => {
        patch({ customBgImage: prevBg });
        message.error(`清除失败：${(e as Error).message}`);
      });
  };

  const handleRestart = () => {
    modal.confirm({
      title: '确认重启服务器？',
      content: '重启后当前会话将断开，需要重新加载页面。',
      okText: '重启',
      okType: 'danger',
      cancelText: '取消',
      onOk: async () => {
        try {
          await systemApi.restartServer();
          message.success('服务器正在重启，请稍候...');
          setTimeout(() => location.reload(), 3000);
        } catch (e) {
          message.error(`重启失败：${(e as Error).message}`);
        }
      },
    });
  };

  /** 立即同步最新设置/数据到桌面小组件（重启浮窗进程以拉取最新值） */
  const [widgetSyncing, setWidgetSyncing] = useState(false);
  const handleWidgetSync = async () => {
    if (!settings?.widgetEnabled) {
      message.info('小组件未启用，请先开启「桌面小组件」');
      return;
    }
    setWidgetSyncing(true);
    try {
      const res = await settingsApi.widgetSync();
      if (res?.synced) {
        message.success('已同步，桌面小组件已刷新');
      } else {
        message.info('小组件未启用，未执行同步');
      }
    } catch (e) {
      message.error(`同步失败：${(e as Error).message}`);
    } finally {
      setWidgetSyncing(false);
    }
  };

  const handleQuit = async () => {
    try {
      await native('quitApp');
    } catch (e) {
      message.error(`退出失败：${(e as Error).message}`);
    }
  };

  const tunnelRefreshTimerRef = useRef<number | null>(null);
  const handleTunnelFieldChange = useCallback(
    <K extends keyof AppSettings>(key: K, value: AppSettings[K]) => {
      void updateField(key, value);
      if (key === 'tunnelToken' || key === 'customDomain') {
        if (tunnelRefreshTimerRef.current) {
          window.clearTimeout(tunnelRefreshTimerRef.current);
        }
        tunnelRefreshTimerRef.current = window.setTimeout(() => {
          if (settings?.tunnelEnabled) {
            void fetchTunnelUrl();
          }
        }, 3000);
      }
    },
    [updateField, fetchTunnelUrl, settings?.tunnelEnabled],
  );

  const handleBump = async (type: 'major' | 'minor' | 'patch') => {
    try {
      const r = await versionApi.bump(type, bumpNotes);
      message.success(`版本已更新：${r.previous} → ${r.version}`);
      setBumpNotes('');
      await fetchVersionInfo();
      await fetchVersionHistory();
    } catch (e) {
      message.error(`版本升级失败：${(e as Error).message}`);
    }
  };

  const handleCheckUpdate = async () => {
    setCheckLoading(true);
    setCheckResult(null);
    try {
      const r = await versionApi.checkUpdate();
      setCheckResult(r);
      if (r.hasUpdate) {
        message.success(`发现新版本：${r.latest}`);
      } else {
        message.info(r.message || '当前已是最新版本');
      }
    } catch (e) {
      message.error(`检查更新失败：${(e as Error).message}`);
    } finally {
      setCheckLoading(false);
    }
  };

  const handleCopyTunnelUrl = async () => {
    if (!tunnelUrl) return;
    try {
      await navigator.clipboard.writeText(tunnelUrl);
      message.success('已复制到剪贴板');
    } catch {
      const ta = document.createElement('textarea');
      ta.value = tunnelUrl;
      document.body.appendChild(ta);
      ta.select();
      document.execCommand('copy');
      document.body.removeChild(ta);
      message.success('已复制到剪贴板');
    }
  };

  const renderAppearance = () => (
    <Space direction="vertical" size={TAB_GAP} style={{ width: '100%' }}>
      <Card style={CARD_STYLE} styles={{ body: CARD_BODY_STYLE }} title="主题模式">
        <Form layout="vertical">
          <Form.Item label={<span style={LABEL_STYLE}>外观模式</span>}>
            <Segmented
              value={storeMode}
              options={[
                { label: '浅色', value: 'light' },
                { label: '深色', value: 'dark' },
              ]}
              onChange={(v) => void handleModeChange(v as 'dark' | 'light')}
              size="large"
            />
          </Form.Item>
        </Form>
      </Card>

      <Card style={CARD_STYLE} styles={{ body: CARD_BODY_STYLE }} title="主题变体">
        <div style={{ display: 'flex', flexWrap: 'wrap', gap: 12 }}>
          {AVAILABLE_THEMES.map((t) => {
            const accent = THEME_ACCENTS[t];
            const selected = storeTheme === t;
            return (
              <div
                key={t}
                onClick={() => handleThemeChange(t)}
                style={{
                  cursor: 'pointer',
                  width: 120,
                  position: 'relative',
                }}
              >
                <div
                  style={{
                    width: 120,
                    height: 80,
                    borderRadius: 10,
                    border: selected ? '2px solid var(--accent)' : '2px solid var(--border-color)',
                    background: `linear-gradient(135deg, ${accent.primary} 0%, ${accent.secondary} 100%)`,
                    boxShadow: selected ? '0 0 0 3px rgba(59,130,246,0.15)' : 'none',
                    transition: 'all 0.2s',
                  }}
                />
                {selected && (
                  <CheckCircleFilled
                    style={{
                      position: 'absolute',
                      top: 4,
                      right: 4,
                      color: 'var(--accent)',
                      fontSize: 20,
                      background: 'rgba(255,255,255,0.9)',
                      borderRadius: '50%',
                    }}
                  />
                )}
                <div
                  style={{
                    textAlign: 'center',
                    marginTop: 6,
                    fontSize: 12,
                    color: selected ? 'var(--accent)' : 'var(--text-secondary)',
                    fontWeight: selected ? 600 : 400,
                  }}
                >
                  {THEME_LABELS[t]}
                </div>
              </div>
            );
          })}
        </div>
      </Card>

      <Card style={CARD_STYLE} styles={{ body: CARD_BODY_STYLE }} title="显示偏好">
        <Form layout="vertical">
          <Form.Item label={<span style={LABEL_STYLE}>默认图表类型</span>}>
            <Radio.Group
              value={settings?.defaultChartType}
              onChange={(e) => void updateField('defaultChartType', e.target.value)}
            >
              <Radio value="bar">柱状图</Radio>
              <Radio value="line">折线图</Radio>
            </Radio.Group>
          </Form.Item>
          <Form.Item label={<span style={LABEL_STYLE}>时间格式</span>}>
            <Radio.Group
              value={settings?.timeFormat}
              onChange={(e) => void updateField('timeFormat', e.target.value)}
            >
              <Radio value="24h">24 小时制</Radio>
              <Radio value="12h">12 小时制</Radio>
            </Radio.Group>
          </Form.Item>
        </Form>
      </Card>

      <Card style={CARD_STYLE} styles={{ body: CARD_BODY_STYLE }} title="自定义背景">
        {settings?.customBgImage ? (
          <Space direction="vertical" size="middle" style={{ width: '100%' }}>
            <div
              style={{
                width: 240,
                height: 160,
                borderRadius: 8,
                border: '1px solid var(--border-color)',
                overflow: 'hidden',
                background: 'var(--bg-card-light)',
              }}
            >
              <img
                src={settings.customBgImage}
                alt="custom-bg"
                style={{ width: '100%', height: '100%', objectFit: 'cover' }}
              />
            </div>
            <Button danger icon={<DeleteOutlined />} onClick={handleClearBg}>
              清除背景
            </Button>
          </Space>
        ) : (
          <Upload.Dragger
            name="file"
            customRequest={handleBgUpload}
            accept="image/*"
            showUploadList={false}
            multiple={false}
          >
            <p className="ant-upload-drag-icon">
              <UploadOutlined />
            </p>
            <p className="ant-upload-text">点击或拖拽图片到此处上传</p>
            <p className="ant-upload-hint">支持 jpg / png / webp，单张不超过 10MB</p>
          </Upload.Dragger>
        )}
      </Card>
    </Space>
  );

  const renderGeneral = () => (
    <Space direction="vertical" size={TAB_GAP} style={{ width: '100%' }}>
      <Card style={CARD_STYLE} styles={{ body: CARD_BODY_STYLE }} title="启动与备份">
        <Form layout="vertical">
          <Form.Item
            label={
              <Space>
                <span style={LABEL_STYLE}>开机自启</span>
                <Text type="secondary" style={{ fontWeight: 400 }}>
                  （{settings?._autoStartRegistered ? '已注册' : '未注册'}）
                </Text>
              </Space>
            }
          >
            <Switch
              checked={settings?.autoStart}
              onChange={(v) => void updateField('autoStart', v)}
            />
          </Form.Item>
          <Form.Item label={<span style={LABEL_STYLE}>自动备份</span>}>
            <Switch
              checked={settings?.autoBackup}
              onChange={(v) => void updateField('autoBackup', v)}
            />
          </Form.Item>
          <Form.Item label={<span style={LABEL_STYLE}>备份保留数量</span>}>
            <InputNumber
              min={1}
              max={100}
              value={settings?.backupCount}
              onChange={(v) => {
                if (v !== null && v !== undefined) {
                  void updateField('backupCount', v);
                }
              }}
              style={{ width: 160 }}
            />
          </Form.Item>
        </Form>
      </Card>

      <Card style={CARD_STYLE} styles={{ body: CARD_BODY_STYLE }} title="桌面与小组件">
        <Form layout="vertical">
          <Form.Item label={<span style={LABEL_STYLE}>无活跃会话自动关闭</span>}>
            <Switch
              checked={settings?.autoCloseIdle}
              onChange={(v) => void updateField('autoCloseIdle', v)}
            />
          </Form.Item>
          <Form.Item label={<span style={LABEL_STYLE}>自动关闭等待（分钟）</span>}>
            <InputNumber
              min={1}
              max={1440}
              disabled={!settings?.autoCloseIdle}
              value={settings?.idleCloseMinutes}
              onChange={(v) => {
                if (v !== null && v !== undefined) {
                  void updateField('idleCloseMinutes', v);
                }
              }}
              style={{ width: 160 }}
            />
          </Form.Item>
          <Form.Item label={<span style={LABEL_STYLE}>桌面小组件</span>}>
            <Space size="middle">
              <Switch
                checked={settings?.widgetEnabled}
                onChange={(v) => void updateField('widgetEnabled', v)}
              />
              <Tooltip title="小组件靠轮询拉取（30s 数据 / 60s 设置），同步会重启浮窗立即拉取最新值">
                <Button
                  type="link"
                  icon={<SyncOutlined />}
                  loading={widgetSyncing}
                  onClick={() => void handleWidgetSync()}
                  style={{ padding: 0 }}
                >
                  同步
                </Button>
              </Tooltip>
            </Space>
          </Form.Item>
        </Form>
      </Card>

      <Card
        style={{ ...CARD_STYLE, borderColor: 'var(--danger-border, rgba(239,68,68,0.4))' }}
        styles={{ body: CARD_BODY_STYLE }}
        title={
          <Space>
            <span style={{ color: 'var(--text-danger)' }}>危险区</span>
            <InfoCircleOutlined style={{ color: 'var(--text-muted)' }} />
          </Space>
        }
      >
        <Space direction="vertical" size="large" style={{ width: '100%' }}>
          <div>
            <Title level={5} style={{ marginTop: 0, marginBottom: 8, color: 'var(--text-primary)' }}>
              重启服务器
            </Title>
            <Text type="secondary" style={{ display: 'block', marginBottom: 12 }}>
              重启后端 HTTP 服务，期间页面会暂时断开。
            </Text>
            <Button
              type="primary"
              danger
              icon={<ReloadOutlined />}
              onClick={handleRestart}
            >
              重启服务器
            </Button>
          </div>
          <div>
            <Title level={5} style={{ marginTop: 0, marginBottom: 8, color: 'var(--text-primary)' }}>
              退出应用
            </Title>
            <Text type="secondary" style={{ display: 'block', marginBottom: 12 }}>
              关闭 BootTracker 窗口并退出后台进程。
            </Text>
            <Button type="primary" icon={<LogoutOutlined />} onClick={handleQuit}>
              退出应用
            </Button>
          </div>
        </Space>
      </Card>
    </Space>
  );

  const renderNetwork = () => (
    <Space direction="vertical" size={TAB_GAP} style={{ width: '100%' }}>
      <Card style={CARD_STYLE} styles={{ body: CARD_BODY_STYLE }} title="局域网访问">
        <Form layout="vertical">
          <Form.Item label={<span style={LABEL_STYLE}>允许局域网设备访问</span>}>
            <Switch
              checked={settings?.lanAccess}
              onChange={(v) => void updateField('lanAccess', v)}
            />
          </Form.Item>
        </Form>
      </Card>

      <Card style={CARD_STYLE} styles={{ body: CARD_BODY_STYLE }} title="外网隧道（Cloudflare）">
        <Form layout="vertical">
          <Form.Item label={<span style={LABEL_STYLE}>启用外网访问隧道</span>}>
            <Switch
              checked={settings?.tunnelEnabled}
              onChange={(v) => {
                void updateField('tunnelEnabled', v);
                if (v) {
                  setTimeout(() => void fetchTunnelUrl(), 500);
                }
              }}
            />
          </Form.Item>
          {settings?.tunnelEnabled && (
            <>
              <Form.Item label={<span style={LABEL_STYLE}>Cloudflare 隧道令牌</span>}>
                <Password
                  placeholder="输入 Cloudflare Zero Trust 隧道令牌"
                  value={settings.tunnelToken}
                  onChange={(e) => handleTunnelFieldChange('tunnelToken', e.target.value)}
                />
              </Form.Item>
              <Form.Item label={<span style={LABEL_STYLE}>自定义域名（可选）</span>}>
                <Input
                  placeholder="例如：boottracker.example.com"
                  value={settings.customDomain}
                  onChange={(e) => handleTunnelFieldChange('customDomain', e.target.value)}
                />
              </Form.Item>
              <Card
                size="small"
                style={{
                  background: 'var(--bg-card-light)',
                  borderColor: 'var(--border-color)',
                  marginBottom: 16,
                }}
                title={
                  <Space>
                    <CloudServerOutlined />
                    <span>设置步骤</span>
                  </Space>
                }
              >
                <ol style={{ margin: 0, paddingLeft: 20, color: 'var(--text-secondary)' }}>
                  <li style={{ marginBottom: 6 }}>
                    前往{' '}
                    <Link href="https://one.dash.cloudflare.com" target="_blank" rel="noopener noreferrer">
                      Cloudflare Zero Trust
                    </Link>{' '}
                    创建账户并登录。
                  </li>
                  <li style={{ marginBottom: 6 }}>
                    在「Networks → Tunnels」中创建一个新 Tunnel，选择「Cloudflared」，复制令牌。
                  </li>
                  <li>
                    将令牌粘贴到上方输入框；如需使用自定义域名，在「Public Hostnames」中添加并填入上方。
                  </li>
                </ol>
              </Card>
              <Form.Item label={<span style={LABEL_STYLE}>外网访问地址</span>}>
                <Space.Compact style={{ width: '100%' }}>
                  <Input
                    value={tunnelUrl ?? ''}
                    placeholder={tunnelUrlLoading ? '获取中...' : '未启动'}
                    readOnly
                    style={{ width: 'calc(100% - 88px)' }}
                    prefix={tunnelUrlLoading ? <Spin size="small" /> : null}
                  />
                  <Tooltip title={tunnelUrl ? '复制地址' : '暂无地址'}>
                    <Button
                      icon={<CopyOutlined />}
                      onClick={handleCopyTunnelUrl}
                      disabled={!tunnelUrl}
                      style={{ width: 88 }}
                    >
                      复制
                    </Button>
                  </Tooltip>
                </Space.Compact>
                {settings.tunnelEnabled && (
                  <Button
                    type="link"
                    size="small"
                    icon={<ReloadOutlined />}
                    onClick={() => void fetchTunnelUrl()}
                    style={{ marginTop: 4, padding: 0 }}
                  >
                    刷新地址
                  </Button>
                )}
              </Form.Item>
            </>
          )}
        </Form>
      </Card>
    </Space>
  );

  const renderAbout = () => (
    <Space direction="vertical" size={TAB_GAP} style={{ width: '100%' }}>
      <Card style={CARD_STYLE} styles={{ body: CARD_BODY_STYLE }} title="应用信息">
        <RowLabelValue
          label="当前版本"
          value={versionInfo?.version ?? '—'}
          valueExtra={<Tag color="blue" style={{ marginLeft: 8 }}>{versionInfo?.appMode ?? '—'}</Tag>}
        />
        <RowLabelValue
          label="运行模式"
          value={
            <Tag color={versionInfo?.appMode === '开发' ? 'orange' : 'green'}>
              {versionInfo?.appMode ?? '—'}
            </Tag>
          }
        />
      </Card>

      <Card style={CARD_STYLE} styles={{ body: CARD_BODY_STYLE }} title="版本管理">
        <Space direction="vertical" size="middle" style={{ width: '100%' }}>
          <div>
            <Text style={{ color: 'var(--text-secondary)', display: 'block', marginBottom: 8 }}>
              升级备注（写入版本历史）
            </Text>
            <Input.TextArea
              value={bumpNotes}
              onChange={(e) => setBumpNotes(e.target.value)}
              placeholder="描述此版本的主要变更..."
              rows={2}
              style={{ marginBottom: 12 }}
            />
          </div>
          <Space wrap>
            <Button icon={<CheckOutlined />} onClick={() => void handleBump('patch')}>
              补丁版本 (x.x.1)
            </Button>
            <Button icon={<ArrowUpOutlined />} onClick={() => void handleBump('minor')}>
              次版本 (x.1.0)
            </Button>
            <Button type="primary" icon={<RocketOutlined />} onClick={() => void handleBump('major')}>
              主版本 (1.0.0)
            </Button>
          </Space>
        </Space>
      </Card>

      <Card style={CARD_STYLE} styles={{ body: CARD_BODY_STYLE }} title="版本历史">
        {historyLoading ? (
          <div style={{ textAlign: 'center', padding: 24 }}>
            <Spin />
          </div>
        ) : versionHistory.length === 0 ? (
          <Text type="secondary">暂无版本历史记录</Text>
        ) : (
          <Timeline
            mode="left"
            items={versionHistory.map((h) => ({
              color: 'var(--accent)',
              label: (
                <Text style={{ color: 'var(--text-muted)', fontFamily: "'JetBrains Mono', monospace", fontSize: 12 }}>
                  {new Date(h.date).toLocaleString('zh-CN')}
                </Text>
              ),
              children: (
                <div>
                  <Space style={{ marginBottom: 4 }}>
                    <Tag color="blue" style={{ fontFamily: "'JetBrains Mono', monospace", fontWeight: 600 }}>
                      v{h.version}
                    </Tag>
                    {h.from && (
                      <Text type="secondary" style={{ fontSize: 12 }}>
                        从 v{h.from}
                      </Text>
                    )}
                  </Space>
                  <div style={{ color: 'var(--text-secondary)', fontSize: 13 }}>
                    {h.notes || <Text type="secondary">（无备注）</Text>}
                  </div>
                </div>
              ),
            }))}
          />
        )}
      </Card>

      <Card style={CARD_STYLE} styles={{ body: CARD_BODY_STYLE }} title="检查更新">
        <Space direction="vertical" size="middle" style={{ width: '100%' }}>
          <Button
            type="primary"
            icon={<DownloadOutlined />}
            onClick={() => void handleCheckUpdate()}
            loading={checkLoading}
          >
            检查更新
          </Button>
          {checkResult && (
            <Card
              size="small"
              style={{
                background: checkResult.hasUpdate
                  ? 'var(--success-bg, rgba(16,185,129,0.08))'
                  : 'var(--bg-card-light)',
                borderColor: 'var(--border-color)',
              }}
            >
              <Space direction="vertical" size="small" style={{ width: '100%' }}>
                <Space>
                  {checkResult.hasUpdate ? (
                    <Tag color="success" icon={<RocketOutlined />}>有新版本</Tag>
                  ) : (
                    <Tag color="default">已是最新</Tag>
                  )}
                  {checkResult.latest && (
                    <Text strong style={{ fontFamily: "'JetBrains Mono', monospace" }}>
                      v{checkResult.latest}
                    </Text>
                  )}
                </Space>
                {checkResult.message && (
                  <Text type="secondary" style={{ fontSize: 13 }}>
                    {checkResult.message}
                  </Text>
                )}
                {checkResult.downloadUrl && (
                  <Link href={checkResult.downloadUrl} target="_blank" rel="noopener noreferrer">
                    <DownloadOutlined /> 前往下载
                  </Link>
                )}
              </Space>
            </Card>
          )}
        </Space>
      </Card>
    </Space>
  );

  return (
    <div style={{ maxWidth: 960, margin: '0 auto' }}>
      <Card
        style={CARD_STYLE}
        styles={{ body: { padding: 0 } }}
        title={
          <Space size="middle" style={{ padding: '12px 24px 0' }}>
            <Title level={4} style={{ margin: 0, color: 'var(--text-primary)' }}>
              系统设置
            </Title>
          </Space>
        }
      >
        {loading ? (
          <div style={{ textAlign: 'center', padding: 64 }}>
            <Spin size="large" />
          </div>
        ) : (
          <Tabs
            defaultActiveKey="appearance"
            size="large"
            style={{ padding: '0 24px 24px' }}
            tabBarStyle={{ marginLeft: -4 }}
            items={[
              {
                key: 'appearance',
                label: '外观',
                children: renderAppearance(),
              },
              {
                key: 'general',
                label: '通用',
                children: renderGeneral(),
              },
              {
                key: 'network',
                label: '网络',
                children: renderNetwork(),
              },
              {
                key: 'about',
                label: '关于',
                children: renderAbout(),
              },
            ]}
          />
        )}
      </Card>
    </div>
  );
}

function RowLabelValue({
  label,
  value,
  valueExtra,
}: {
  label: string;
  value: React.ReactNode;
  valueExtra?: React.ReactNode;
}) {
  return (
    <div
      style={{
        display: 'flex',
        justifyContent: 'space-between',
        alignItems: 'center',
        padding: '10px 0',
        borderBottom: '1px solid var(--border-light)',
      }}
    >
      <Text style={{ color: 'var(--text-muted)' }}>{label}</Text>
      <Space>
        {typeof value === 'string' ? (
          <Text strong style={{ color: 'var(--text-primary)', fontFamily: "'JetBrains Mono', monospace" }}>
            {value}
          </Text>
        ) : (
          value
        )}
        {valueExtra}
      </Space>
    </div>
  );
}

export default Settings;
