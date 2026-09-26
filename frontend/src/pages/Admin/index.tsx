import { useCallback, useEffect, useState } from 'react';
import {
  App as AntdApp,
  Badge,
  Button,
  Card,
  Col,
  DatePicker,
  Divider,
  Form,
  Layout,
  Modal,
  Popconfirm,
  Row,
  Space,
  Statistic,
  Table,
  Tabs,
  Tooltip,
  Typography,
} from 'antd';
import type { ColumnsType, TableRowSelection } from 'antd/es/table/interface';
import {
  DeleteOutlined,
  EditOutlined,
  FileExcelOutlined,
  FileTextOutlined,
  PlusOutlined,
  ReloadOutlined,
  RollbackOutlined,
  SaveOutlined,
  WarningOutlined,
} from '@ant-design/icons';
import dayjs, { Dayjs } from 'dayjs';

import { useBootData } from '../../hooks/useBootData';
import { fmtFullTime, fmtDuration } from '../../stores/sessionStore';
import { dataApi } from '../../api/data';
import { backupApi } from '../../api/backup';
import { trashApi, softDeleteSession } from '../../api/trash';
import { statsApi } from '../../api/stats';
import type { BootSession, OverviewStats, TrashData } from '../../api/types';
import StatusPill from '../../components/StatusPill';
import LiveDuration from '../../components/LiveDuration';

const { Title, Text } = Typography;

/* LiveDuration 调用点的稳定 style 引用：避免每次父渲染生成新对象使 memo 失效 */
const LIVE_CELL_STYLE: React.CSSProperties = {
  fontFamily: 'JetBrains Mono, monospace',
  color: 'var(--accent)',
};

/* ================= 导出工具（与 Records 共享行为，自包含副本） ================= */

function pad2(n: number): string {
  return String(n).padStart(2, '0');
}

function todayFilenameStamp(): string {
  const d = new Date();
  return `${d.getFullYear()}${pad2(d.getMonth() + 1)}${pad2(d.getDate())}`;
}

function triggerDownload(blob: Blob, filename: string): void {
  const url = URL.createObjectURL(blob);
  const a = document.createElement('a');
  a.href = url;
  a.download = filename;
  document.body.appendChild(a);
  a.click();
  document.body.removeChild(a);
  URL.revokeObjectURL(url);
}

function escapeCSVField(v: string | number | null | undefined): string {
  if (v === null || v === undefined) return '';
  const s = String(v);
  if (s.includes(',') || s.includes('"') || s.includes('\n') || s.includes('\r')) {
    return `"${s.replace(/"/g, '""')}"`;
  }
  return s;
}

function exportCSV(sessions: BootSession[]): void {
  const header = ['开机时间', '关机时间', '时长(ms)', '时长字符串'];
  const rows = sessions.map((s) => {
    const bootStr = fmtFullTime(s.bootTime);
    const shutdownStr = s.shutdownTime ? fmtFullTime(s.shutdownTime) : '';
    const durationMs = s.duration ?? '';
    const durationStr = s.duration ? fmtDuration(s.duration) : '';
    return [bootStr, shutdownStr, durationMs, durationStr].map(escapeCSVField).join(',');
  });
  const csv = '\ufeff' + [header.join(','), ...rows].join('\r\n');
  const blob = new Blob([csv], { type: 'text/csv;charset=utf-8;' });
  triggerDownload(blob, `boot-records-${todayFilenameStamp()}.csv`);
}

/* ================= 页面主组件 ================= */

function Admin() {
  const { message, modal } = AntdApp.useApp();
  const { data, loading, refresh } = useBootData();

  /* ---------- 状态 ---------- */
  const [activeTab, setActiveTab] = useState('manage');

  const [selectedRowKeys, setSelectedRowKeys] = useState<React.Key[]>([]);
  const [editingModalOpen, setEditingModalOpen] = useState(false);
  const [editingSession, setEditingSession] = useState<BootSession | null>(null);
  const [editForm] = Form.useForm<{ bootTime: Dayjs; shutdownTime?: Dayjs | null }>();

  const [addForm] = Form.useForm<{ bootTime: Dayjs; shutdownTime?: Dayjs | null }>();

  const [trashData, setTrashData] = useState<TrashData>({ sessions: [] });
  const [trashLoading, setTrashLoading] = useState(false);

  const [backupList, setBackupList] = useState<string[]>([]);
  const [backupLoading, setBackupLoading] = useState(false);

  const [overview, setOverview] = useState<OverviewStats | null>(null);
  const [overviewLoading, setOverviewLoading] = useState(false);

  /* ---------- 加载附属数据（组件挂载即加载） ---------- */
  const loadTrash = useCallback(async () => {
    setTrashLoading(true);
    try {
      const t = await trashApi.get();
      setTrashData(t);
    } catch (e) {
      message.error(`加载回收站失败：${(e as Error).message}`);
    } finally {
      setTrashLoading(false);
    }
  }, [message]);

  const loadBackups = useCallback(async () => {
    setBackupLoading(true);
    try {
      const r = await backupApi.list();
      setBackupList(r.backups ?? []);
    } catch (e) {
      message.error(`加载备份列表失败：${(e as Error).message}`);
    } finally {
      setBackupLoading(false);
    }
  }, [message]);

  const loadOverview = useCallback(async () => {
    setOverviewLoading(true);
    try {
      const r = await statsApi.overview();
      setOverview(r);
    } catch (e) {
      message.error(`加载统计失败：${(e as Error).message}`);
    } finally {
      setOverviewLoading(false);
    }
  }, [message]);

  useEffect(() => {
    void loadTrash();
    void loadBackups();
    void loadOverview();
  }, [loadTrash, loadBackups, loadOverview]);

  /* ---------- 表格数据：按开机时间倒序 ---------- */
  const sortedSessions = [...data.sessions].sort(
    (a, b) => new Date(b.bootTime).getTime() - new Date(a.bootTime).getTime(),
  );

  /* ---------- 表格列（结构化：数字右对齐 tabular-nums + 状态胶囊） ---------- */
  const baseColumns: ColumnsType<BootSession> = [
    {
      title: '序号',
      key: 'idx',
      width: 70,
      align: 'right',
      render: (_v, _r, idx) => (
        <span className="tnum" style={{ color: 'var(--text-muted)' }}>
          {idx + 1}
        </span>
      ),
    },
    {
      title: '开机时间',
      dataIndex: 'bootTime',
      key: 'bootTime',
      width: 190,
      render: (v: string) => (
        <span className="tnum" style={{ fontFamily: 'JetBrains Mono, monospace' }}>
          {fmtFullTime(v)}
        </span>
      ),
    },
    {
      title: '关机时间',
      dataIndex: 'shutdownTime',
      key: 'shutdownTime',
      width: 190,
      render: (v: string | null) =>
        v ? (
          <span className="tnum" style={{ fontFamily: 'JetBrains Mono, monospace' }}>
            {fmtFullTime(v)}
          </span>
        ) : (
          <span style={{ color: 'var(--text-muted)' }}>—</span>
        ),
    },
    {
      title: '会话时长',
      key: 'duration',
      width: 150,
      align: 'right',
      render: (_v, record) =>
        record.shutdownTime ? (
          <span className="tnum" style={{ fontFamily: 'JetBrains Mono, monospace' }}>
            {fmtDuration(record.duration)}
          </span>
        ) : (
          <LiveDuration
            bootTime={record.bootTime}
            className="tnum"
            style={LIVE_CELL_STYLE}
          />
        ),
    },
    {
      title: '状态',
      key: 'status',
      width: 100,
      render: (_v, record) =>
        record.shutdownTime ? (
          <StatusPill tone="success">已关机</StatusPill>
        ) : (
          <StatusPill tone="accent">进行中</StatusPill>
        ),
    },
  ];

  /* ---------- 管理列（带编辑/删除） ---------- */
  const openEditModal = (s: BootSession) => {
    setEditingSession(s);
    editForm.setFieldsValue({
      bootTime: dayjs(s.bootTime),
      shutdownTime: s.shutdownTime ? dayjs(s.shutdownTime) : null,
    });
    setEditingModalOpen(true);
  };

  const handleEditSubmit = async () => {
    if (!editingSession) return;
    try {
      const values = await editForm.validateFields();
      const patch: Partial<Pick<BootSession, 'bootTime' | 'shutdownTime'>> = {
        bootTime: values.bootTime.toISOString(),
        shutdownTime: values.shutdownTime ? values.shutdownTime.toISOString() : null,
      };
      await dataApi.updateSession(editingSession.id, patch);
      message.success('更新成功');
      setEditingModalOpen(false);
      setEditingSession(null);
      editForm.resetFields();
      await refresh();
    } catch (e) {
      if ((e as Error).name !== 'ValidateError') {
        message.error(`更新失败：${(e as Error).message}`);
      }
    }
  };

  const handleSoftDelete = async (s: BootSession) => {
    try {
      await softDeleteSession(s.id, {
        fetchData: () => dataApi.getData(),
        saveData: (d) => dataApi.saveData(d),
        fetchTrash: () => trashApi.get(),
        saveTrash: (t) => trashApi.save(t),
      });
      message.success('已移至回收站');
      setSelectedRowKeys((keys) => keys.filter((k) => k !== s.id));
      await Promise.all([refresh(), loadTrash()]);
    } catch (e) {
      message.error(`删除失败：${(e as Error).message}`);
    }
  };

  const handleClearAll = () => {
    const list = data.sessions;
    modal.confirm({
      title: '永久删除全部记录？',
      icon: <WarningOutlined style={{ color: '#ff4d4f' }} />,
      content: (
        <div>
          <p>所有开机记录将被永久删除，无法恢复（不进入回收站）。</p>
          <div
            style={{
              marginTop: 8,
              padding: '8px 12px',
              borderRadius: 8,
              background: 'var(--bg-card-light)',
              border: '1px solid var(--border-light)',
              fontSize: 13,
            }}
          >
            {list.length > 0 ? (
              <>
                将删除 <b className="tnum">{list.length}</b> 条记录（
                {fmtFullTime(
                  list.reduce((min, s) => (s.bootTime < min ? s.bootTime : min), list[0].bootTime),
                )}
                {' ～ '}
                {fmtFullTime(
                  list.reduce((max, s) => (s.bootTime > max ? s.bootTime : max), list[0].bootTime),
                )}
                ）
              </>
            ) : (
              '当前没有记录'
            )}
          </div>
        </div>
      ),
      okText: `永久删除 ${list.length} 条记录`,
      okType: 'danger',
      cancelText: '取消',
      onOk: async () => {
        try {
          await dataApi.clearData();
          message.success('已清空全部记录');
          setSelectedRowKeys([]);
          await Promise.all([refresh(), loadOverview()]);
        } catch (e) {
          message.error(`清空失败：${(e as Error).message}`);
        }
      },
    });
  };

  const handleMergeSelected = async () => {
    if (selectedRowKeys.length < 2) {
      message.warning('请至少选择 2 条记录');
      return;
    }
    try {
      await dataApi.mergeSessions(selectedRowKeys.map(String));
      message.success(`已合并 ${selectedRowKeys.length} 条记录`);
      setSelectedRowKeys([]);
      await Promise.all([refresh(), loadOverview(), loadTrash()]);
    } catch (e) {
      message.error(`合并失败：${(e as Error).message}`);
    }
  };

  const manageColumns: ColumnsType<BootSession> = [
    ...baseColumns,
    {
      title: '操作',
      key: 'action',
      width: 150,
      fixed: 'right',
      render: (_v, record) => (
        <Space size="small">
          <Button
            type="link"
            size="small"
            icon={<EditOutlined />}
            onClick={() => openEditModal(record)}
          >
            编辑
          </Button>
          <Popconfirm
            title="移入回收站？"
            description={`开机于 ${fmtFullTime(record.bootTime)} 的记录将移入回收站，可还原。`}
            okText="移入回收站"
            okType="danger"
            cancelText="取消"
            onConfirm={() => void handleSoftDelete(record)}
          >
            <Button type="link" size="small" danger icon={<DeleteOutlined />}>
              删除
            </Button>
          </Popconfirm>
        </Space>
      ),
    },
  ];

  const manageRowSelection: TableRowSelection<BootSession> = {
    selectedRowKeys,
    onChange: (keys) => setSelectedRowKeys(keys),
  };

  /* ---------- 回收站操作 ---------- */
  const handleRestoreTrash = async (id: string) => {
    try {
      await trashApi.restore(id);
      message.success('已还原');
      await Promise.all([refresh(), loadTrash(), loadOverview()]);
    } catch (e) {
      message.error(`还原失败：${(e as Error).message}`);
    }
  };

  const handlePermanentDelete = async (id: string) => {
    try {
      await trashApi.permanentDelete(id);
      message.success('已永久删除');
      await loadTrash();
    } catch (e) {
      message.error(`删除失败：${(e as Error).message}`);
    }
  };

  const handleClearTrash = () => {
    const list = trashData.sessions;
    modal.confirm({
      title: '永久删除回收站内容？',
      icon: <WarningOutlined style={{ color: '#ff4d4f' }} />,
      content: (
        <div>
          <p>回收站中的所有记录将被永久删除，无法恢复。</p>
          <div
            style={{
              marginTop: 8,
              padding: '8px 12px',
              borderRadius: 8,
              background: 'var(--bg-card-light)',
              border: '1px solid var(--border-light)',
              fontSize: 13,
            }}
          >
            {list.length > 0 ? (
              <>
                将删除 <b className="tnum">{list.length}</b> 条记录（最早：
                {fmtFullTime(
                  list.reduce((min, s) => (s.bootTime < min ? s.bootTime : min), list[0].bootTime),
                )}
                ）
              </>
            ) : (
              '回收站是空的'
            )}
          </div>
        </div>
      ),
      okText: `永久删除 ${list.length} 条`,
      okType: 'danger',
      cancelText: '取消',
      onOk: async () => {
        try {
          await trashApi.clear();
          message.success('回收站已清空');
          await loadTrash();
        } catch (e) {
          message.error(`清空失败：${(e as Error).message}`);
        }
      },
    });
  };

  const trashColumns: ColumnsType<BootSession> = [
    ...baseColumns,
    {
      title: '操作',
      key: 'action',
      width: 180,
      fixed: 'right',
      render: (_v, record) => (
        <Space size="small">
          <Button
            type="link"
            size="small"
            icon={<RollbackOutlined />}
            onClick={() => void handleRestoreTrash(record.id)}
          >
            还原
          </Button>
          <Popconfirm
            title="永久删除这条记录？"
            description={`开机于 ${fmtFullTime(record.bootTime)}，删除后无法恢复。`}
            okText="永久删除"
            okType="danger"
            cancelText="取消"
            onConfirm={() => void handlePermanentDelete(record.id)}
          >
            <Button type="link" size="small" danger icon={<DeleteOutlined />}>
              永久删除
            </Button>
          </Popconfirm>
        </Space>
      ),
    },
  ];

  /* ---------- 添加记录 ---------- */
  const handleAddSubmit = async () => {
    try {
      const values = await addForm.validateFields();
      await dataApi.addSession(
        values.bootTime.toISOString(),
        values.shutdownTime ? values.shutdownTime.toISOString() : undefined,
      );
      message.success('已添加记录');
      addForm.resetFields();
      await Promise.all([refresh(), loadOverview()]);
    } catch (e) {
      if ((e as Error).name !== 'ValidateError') {
        message.error(`添加失败：${(e as Error).message}`);
      }
    }
  };

  /* ---------- 备份操作 ---------- */
  const handleRestoreBackup = (filename: string) => {
    modal.confirm({
      title: `恢复备份：${filename}？`,
      icon: <WarningOutlined style={{ color: '#faad14' }} />,
      content: '当前数据将被备份内容覆盖，此操作不可撤销。',
      okText: '确认恢复',
      okType: 'primary',
      cancelText: '取消',
      onOk: async () => {
        try {
          await backupApi.restore(filename);
          message.success('备份已恢复');
          await Promise.all([refresh(), loadOverview(), loadTrash()]);
        } catch (e) {
          message.error(`恢复失败：${(e as Error).message}`);
        }
      },
    });
  };

  const handleDeleteBackup = (filename: string) => {
    modal.confirm({
      title: `删除备份：${filename}？`,
      content: '备份文件将被永久删除。',
      okText: '删除',
      okType: 'danger',
      cancelText: '取消',
      onOk: async () => {
        try {
          await backupApi.delete(filename);
          message.success('备份已删除');
          await loadBackups();
        } catch (e) {
          message.error(`删除失败：${(e as Error).message}`);
        }
      },
    });
  };

  const handleCleanBackups = () => {
    modal.confirm({
      title: '清理旧备份？',
      content: '将保留最近 10 份备份，其余删除。',
      okText: '确认清理',
      cancelText: '取消',
      onOk: async () => {
        try {
          const r = await backupApi.clean(10);
          message.success(`已清理 ${r.deleted} 份，剩余 ${r.remaining} 份`);
          await loadBackups();
        } catch (e) {
          message.error(`清理失败：${(e as Error).message}`);
        }
      },
    });
  };

  /* ---------- 渲染 ---------- */
  return (
    <Layout.Content style={{ padding: '24px', maxWidth: 1400, margin: '0 auto', width: '100%' }}>
      {/* 顶部标题 + 刷新 */}
      <Space style={{ marginBottom: 16, width: '100%', justifyContent: 'space-between' }} wrap>
        <Title level={4} style={{ color: 'var(--text-primary)', margin: 0 }}>
          管理
        </Title>
        <Space wrap>
          <Button
            icon={<ReloadOutlined />}
            onClick={() => void Promise.all([refresh(), loadTrash(), loadBackups(), loadOverview()])}
            loading={loading}
          >
            刷新全部
          </Button>
          <Space.Compact>
            <Button icon={<FileTextOutlined />} onClick={() => exportCSV(sortedSessions)}>
              导出 CSV
            </Button>
            <Tooltip title="如需请安装 xlsx 包">
              <Button icon={<FileExcelOutlined />} disabled>
                导出 XLSX
              </Button>
            </Tooltip>
          </Space.Compact>
        </Space>
      </Space>

      <Card
        style={{
          background: 'var(--bg-card)',
          borderColor: 'var(--border-color)',
          marginBottom: 16,
        }}
        styles={{ body: { padding: 16 } }}
      >
        {/* 4 张统计卡片 */}
        <Row gutter={[16, 16]} style={{ marginBottom: 16 }}>
          <Col xs={24} sm={12} lg={6}>
            <Card size="small" loading={overviewLoading}>
              <Statistic
                title="总开机次数"
                value={overview?.totalBoot ?? 0}
                prefix={<SaveOutlined style={{ color: '#1677ff' }} />}
                valueStyle={{ color: '#1677ff' }}
              />
            </Card>
          </Col>
          <Col xs={24} sm={12} lg={6}>
            <Card size="small" loading={overviewLoading}>
              <Statistic
                title="总关机次数"
                value={overview?.totalShutdown ?? 0}
                prefix={<RollbackOutlined style={{ color: '#52c41a' }} />}
                valueStyle={{ color: '#52c41a' }}
              />
            </Card>
          </Col>
          <Col xs={24} sm={12} lg={6}>
            <Card size="small" loading={overviewLoading}>
              <Statistic
                title="进行中会话"
                value={overview?.activeCount ?? 0}
                prefix={<Badge status="processing" />}
                valueStyle={{ color: '#13c2c2' }}
              />
            </Card>
          </Col>
          <Col xs={24} sm={12} lg={6}>
            <Card size="small" loading={overviewLoading}>
              <Statistic
                title="平均时长"
                value={fmtDuration(overview?.avgDuration)}
                prefix={<FileTextOutlined style={{ color: '#722ed1' }} />}
              />
            </Card>
          </Col>
        </Row>

        <Divider style={{ margin: '8px 0 16px 0' }} />

        <Tabs
          activeKey={activeTab}
          onChange={setActiveTab}
          items={[
            {
              key: 'manage',
              label: '记录管理',
              children: (
                <>
                  <Space style={{ marginBottom: 12 }} wrap>
                    <Button
                      type="primary"
                      onClick={() => void handleMergeSelected()}
                      disabled={selectedRowKeys.length < 2}
                      icon={<SaveOutlined />}
                    >
                      合并选中（{selectedRowKeys.length}）
                    </Button>
                    <Button danger icon={<DeleteOutlined />} onClick={handleClearAll}>
                      清空全部
                    </Button>
                    <Button icon={<ReloadOutlined />} onClick={() => void refresh()} loading={loading}>
                      刷新记录
                    </Button>
                  </Space>
                  <Table<BootSession>
                    rowKey="id"
                    loading={loading}
                    dataSource={sortedSessions}
                    columns={manageColumns}
                    rowSelection={manageRowSelection}
                    scroll={{ x: 1000 }}
                    pagination={{
                      pageSize: 15,
                      showSizeChanger: true,
                      showTotal: (t) => `共 ${t} 条`,
                      pageSizeOptions: ['10', '15', '30', '50', '100'],
                    }}
                  />
                </>
              ),
            },
            {
              key: 'trash',
              label: `回收站（${trashData.sessions.length}）`,
              children: (
                <>
                  <Space style={{ marginBottom: 12 }}>
                    <Button danger icon={<DeleteOutlined />} onClick={handleClearTrash}>
                      清空回收站
                    </Button>
                    <Button icon={<ReloadOutlined />} onClick={() => void loadTrash()} loading={trashLoading}>
                      刷新
                    </Button>
                  </Space>
                  <Table<BootSession>
                    rowKey="id"
                    loading={trashLoading}
                    dataSource={trashData.sessions}
                    columns={trashColumns}
                    scroll={{ x: 1000 }}
                    pagination={{
                      pageSize: 15,
                      showSizeChanger: true,
                      showTotal: (t) => `共 ${t} 条`,
                      pageSizeOptions: ['10', '15', '30', '50', '100'],
                    }}
                  />
                </>
              ),
            },
            {
              key: 'add',
              label: '添加记录',
              children: (
                <Card size="small" style={{ maxWidth: 560 }}>
                  <Form
                    form={addForm}
                    layout="vertical"
                    onFinish={() => void handleAddSubmit()}
                  >
                    <Form.Item
                      name="bootTime"
                      label="开机时间"
                      rules={[{ required: true, message: '请选择开机时间' }]}
                    >
                      <DatePicker showTime style={{ width: '100%' }} placeholder="选择开机时间" />
                    </Form.Item>
                    <Form.Item name="shutdownTime" label="关机时间（可选）">
                      <DatePicker showTime style={{ width: '100%' }} placeholder="关机时间，留空表示进行中" allowClear />
                    </Form.Item>
                    <Form.Item>
                      <Space>
                        <Button type="primary" htmlType="submit" icon={<PlusOutlined />}>
                          添加记录
                        </Button>
                        <Button onClick={() => addForm.resetFields()}>重置</Button>
                      </Space>
                    </Form.Item>
                  </Form>
                </Card>
              ),
            },
            {
              key: 'backup',
              label: '备份还原',
              children: (
                <>
                  <Space style={{ marginBottom: 12 }} wrap>
                    <Button icon={<ReloadOutlined />} onClick={() => void loadBackups()} loading={backupLoading}>
                      刷新列表
                    </Button>
                    <Button onClick={handleCleanBackups} icon={<DeleteOutlined />}>
                      清理旧备份（保留最近 10）
                    </Button>
                  </Space>
                  <Table<string>
                    rowKey={(r) => r}
                    loading={backupLoading}
                    dataSource={backupList}
                    columns={[
                      {
                        title: '备份文件名',
                        dataIndex: 'name',
                        key: 'name',
                        render: (v: string) => <Text code>{v}</Text>,
                      },
                      {
                        title: '操作',
                        key: 'action',
                        width: 200,
                        render: (_v, record) => (
                          <Space size="small">
                            <Button
                              type="link"
                              size="small"
                              icon={<RollbackOutlined />}
                              onClick={() => handleRestoreBackup(record)}
                            >
                              恢复
                            </Button>
                            <Button
                              type="link"
                              size="small"
                              danger
                              icon={<DeleteOutlined />}
                              onClick={() => handleDeleteBackup(record)}
                            >
                              删除
                            </Button>
                          </Space>
                        ),
                      },
                    ]}
                    pagination={{
                      pageSize: 15,
                      showSizeChanger: true,
                      showTotal: (t) => `共 ${t} 条`,
                      pageSizeOptions: ['10', '15', '30', '50', '100'],
                    }}
                    locale={{ emptyText: '暂无备份' }}
                  />
                </>
              ),
            },
          ]}
        />
      </Card>

      {/* 编辑 Modal */}
      <Modal
        title="编辑开机记录"
        open={editingModalOpen}
        onOk={() => void handleEditSubmit()}
        onCancel={() => {
          setEditingModalOpen(false);
          setEditingSession(null);
          editForm.resetFields();
        }}
        okText="保存"
        cancelText="取消"
        destroyOnHidden
      >
        <Form form={editForm} layout="vertical" preserve={false}>
          <Form.Item
            name="bootTime"
            label="开机时间"
            rules={[{ required: true, message: '请选择开机时间' }]}
          >
            <DatePicker showTime style={{ width: '100%' }} placeholder="开机时间" />
          </Form.Item>
          <Form.Item name="shutdownTime" label="关机时间">
            <DatePicker showTime style={{ width: '100%' }} placeholder="关机时间，留空表示进行中" allowClear />
          </Form.Item>
        </Form>
      </Modal>
    </Layout.Content>
  );
}

export default Admin;
