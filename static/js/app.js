const STORAGE_KEY = 'bootTrackerData';
const FILE_STORAGE_KEY = 'bootTrackerData_file';

/* ——— 文件存储桥接（通过 HTTP 接口与 Python 通信）——— */
var _serverPort = null;

function _detectServerPort() {
  // 从 URL hash 读取端口：index.html#port=18792
  var hash = window.location.hash;
  var m = hash.match(/port=(\d+)/);
  if (m) return parseInt(m[1], 10);
  return null;
}

function _isServerAvailable() {
  return _serverPort !== null;
}

function _serverUrl(path) {
  return 'http://127.0.0.1:' + _serverPort + path;
}

/* ——— 数据加载（优先文件服务器，回退 localStorage）——— */
function loadData() {
  // 同步返回：返回缓存或 localStorage
  if (window._bootDataCache) return JSON.parse(JSON.stringify(window._bootDataCache));

  var raw = localStorage.getItem(STORAGE_KEY);
  if (raw) {
    try { var d = JSON.parse(raw); window._bootDataCache = d; return JSON.parse(JSON.stringify(d)); } catch(e) {}
  }
  return { bootCount: 0, shutdownCount: 0, sessions: [] };
}

/* 异步加载数据（桌面模式必须等服务器就绪，纯浏览器用 localStorage） */
function loadDataAsync(callback, retries) {
  if (_isServerAvailable()) {
    // 桌面模式：重试直到服务器响应（最多等待 10 秒）
    var maxRetries = retries !== undefined ? retries : 50;
    fetch(_serverUrl('/api/data'))
      .then(function(r) { return r.json(); })
      .then(function(data) {
        if (data && typeof data === 'object' && !data.error) {
          window._bootDataCache = data;
          window._serverReady = true;
          try { localStorage.setItem(STORAGE_KEY, JSON.stringify(data)); localStorage.setItem(FILE_STORAGE_KEY, '1'); } catch(e) {}
          callback(data);
        } else {
          callback(loadData());
        }
      })
      .catch(function() {
        if (maxRetries > 0) {
          setTimeout(function() { loadDataAsync(callback, maxRetries - 1); }, 200);
        } else {
          // 服务器始终无响应，回退 localStorage
          callback(loadData());
        }
      });
  } else {
    callback(loadData());
  }
}

/* ——— 数据保存（优先文件服务器，同步写入 localStorage）——— */
function saveData(data) {
  // 始终同步写入 localStorage
  try { localStorage.setItem(STORAGE_KEY, JSON.stringify(data)); } catch(e) {}
  window._bootDataCache = JSON.parse(JSON.stringify(data));

  // 异步写入文件服务器（仅桌面模式）
  if (_isServerAvailable() && window._serverReady) {
    fetch(_serverUrl('/api/data'), {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(data)
    }).catch(function() {});
  }
}

function genId() {
  return Date.now().toString(36) + Math.random().toString(36).slice(2, 8);
}

function fmtTime(iso) {
  if (!iso) return '—';
  const d = new Date(iso);
  return d.toLocaleString('zh-CN', {
    month: '2-digit', day: '2-digit',
    hour: '2-digit', minute: '2-digit', second: '2-digit'
  });
}

function fmtFullTime(iso) {
  if (!iso) return '—';
  const d = new Date(iso);
  return d.toLocaleString('zh-CN', {
    year: 'numeric', month: '2-digit', day: '2-digit',
    hour: '2-digit', minute: '2-digit', second: '2-digit'
  });
}

function fmtDuration(ms) {
  if (!ms || ms <= 0) return '—';
  const s = Math.floor(ms / 1000);
  const h = Math.floor(s / 3600);
  const m = Math.floor((s % 3600) / 60);
  const sec = s % 60;
  if (h > 0) return h + '时' + m + '分' + sec + '秒';
  if (m > 0) return m + '分' + sec + '秒';
  return sec + '秒';
}

// ——— 开机记录 ———
function initBoot() {
  // 优先从文件异步加载数据，再初始化
  loadDataAsync(function(data) {
    window._bootDataCache = data;  // 确保缓存更新
    _doInitBoot(data);
  });
}

function _doInitBoot(data) {
  let currentId = sessionStorage.getItem('currentSessionId');

  // 检查当前会话是否仍然有效
  if (currentId) {
    const existing = data.sessions.find(s => s.id === currentId);
    if (existing && !existing.shutdownTime) {
      updateMainUI(data, existing);
      startTimer(existing.bootTime);
      refreshMainTable();
      refreshChart();
      renderHeatmap();
      return;
    }
    sessionStorage.removeItem('currentSessionId');
    currentId = null;
  }

  // 后台服务模式：检查数据中是否已有活跃会话（关闭窗口后重新打开的情况）
  var activeSession = data.sessions.find(function(s) { return !s.shutdownTime; });
  if (activeSession) {
    currentId = activeSession.id;
    sessionStorage.setItem('currentSessionId', currentId);
    updateMainUI(data, activeSession);
    startTimer(activeSession.bootTime);
    refreshMainTable();
    refreshChart();
    renderHeatmap();
    return;
  }

  // 没有活跃会话
  // 桌面模式：不应该发生（Python 后端开机时已创建），展示已关机状态
  if (_isServerAvailable()) {
    updateMainUI(data, null);
    refreshMainTable();
    refreshChart();
    renderHeatmap();
    return;
  }

  // 纯浏览器模式：才由 JS 创建新会话
  const now = new Date().toISOString();
  currentId = genId();
  data.bootCount = (data.bootCount || 0) + 1;
  data.sessions.push({
    id: currentId,
    bootTime: now,
    shutdownTime: null,
    duration: null
  });
  sessionStorage.setItem('currentSessionId', currentId);
  saveData(data);
  updateMainUI(data, data.sessions.find(s => s.id === currentId));
  startTimer(now);
  refreshMainTable();
  setTimeout(refreshChart, 100);
  setTimeout(renderHeatmap, 200);
  setTimeout(refreshChart, 600);
}

function animateNumber(el, target) {
  var current = parseInt(el.textContent, 10);
  if (isNaN(current)) current = 0;
  if (current === target) { el.textContent = target; return; }
  var step = target > current ? 1 : -1;
  var interval = Math.max(30, 400 / Math.abs(target - current));
  var timer = setInterval(function() {
    current += step;
    el.textContent = current;
    if (current === target) clearInterval(timer);
  }, interval);
}

function updateMainUI(data, currentSession) {
  var el = document.getElementById('bootCount');
  if (el) el.textContent = data.bootCount || 0;
  var sc = document.getElementById('shutdownCount');
  if (sc) sc.textContent = data.shutdownCount || 0;
  document.getElementById('currentBootTime').textContent = currentSession ? fmtFullTime(currentSession.bootTime) : '—';

  const btn = document.getElementById('btnShutdown');
  if (!currentSession || currentSession.shutdownTime) {
    document.getElementById('currentStatus').innerHTML = '<span class="status-dot closed"></span>已关机';
    btn.disabled = true;
    btn.textContent = '已记录关机';
  } else {
    document.getElementById('currentStatus').innerHTML = '<span class="status-dot active"></span>运行中';
    btn.disabled = false;
    btn.textContent = '记录关机';
  }
}

let timerInterval = null;
function startTimer(bootIso) {
  if (timerInterval) clearInterval(timerInterval);
  function update() {
    const now = new Date();
    const boot = new Date(bootIso);
    document.getElementById('runningTime').textContent = fmtDuration(now - boot);
    document.getElementById('navTime').textContent = now.toLocaleTimeString('zh-CN', { hour: '2-digit', minute: '2-digit', second: '2-digit' });
  }
  update();
  timerInterval = setInterval(update, 1000);
}

// ——— 关机记录 ———
function recordShutdown() {
  const data = loadData();
  const currentId = sessionStorage.getItem('currentSessionId');
  if (!currentId) return;

  const session = data.sessions.find(s => s.id === currentId);
  if (!session || session.shutdownTime) return;

  const now = new Date();
  session.shutdownTime = now.toISOString();
  session.duration = now - new Date(session.bootTime);
  data.shutdownCount = (data.shutdownCount || 0) + 1;

  saveData(data);
  updateMainUI(data, session);
  // 停止实时计时器，固定显示最终时长
  if (timerInterval) { clearInterval(timerInterval); timerInterval = null; }
  document.getElementById('runningTime').textContent = fmtDuration(session.duration);
  refreshMainTable();
  refreshChart();
  renderHeatmap();
}

// ——— 主界面表格刷新 ———
function refreshMainTable() {
  const data = loadData();
  const tbody = document.getElementById('mainRecordsBody');
  const empty = document.getElementById('mainEmptyMsg');
  const sorted = [...data.sessions].reverse();

  if (sorted.length === 0) {
    tbody.innerHTML = '';
    empty.style.display = 'block';
    return;
  }
  empty.style.display = 'none';
  tbody.innerHTML = sorted.map((s, i) => {
    const idx = data.sessions.length - i;
    const isActive = !s.shutdownTime;
    return '<tr>' +
      '<td>' + idx + '</td>' +
      '<td>' + fmtTime(s.bootTime) + '</td>' +
      '<td>' + (isActive ? '—' : fmtTime(s.shutdownTime)) + '</td>' +
      '<td>' + (isActive ? '—' : fmtDuration(s.duration)) + '</td>' +
      '<td>' + (isActive
        ? '<span class="status-dot active"></span>进行中'
        : '<span class="status-dot closed"></span>已关机') + '</td>' +
    '</tr>';
  }).join('');
}

// ——— 管理员面板 ———
function toggleAdmin() {
  const overlay = document.getElementById('adminOverlay');
  overlay.classList.toggle('active');
  if (overlay.classList.contains('active')) refreshAdmin();
}

function refreshAdmin() {
  const data = loadData();
  document.getElementById('statBoots').textContent = data.bootCount || 0;
  document.getElementById('statShutdowns').textContent = data.shutdownCount || 0;

  const active = data.sessions.filter(s => !s.shutdownTime).length;
  document.getElementById('statActive').textContent = active;

  // 平均时长
  const closed = data.sessions.filter(s => s.duration);
  if (closed.length > 0) {
    const avg = closed.reduce(function(a, s) { return a + s.duration; }, 0) / closed.length;
    document.getElementById('statAvg').textContent = fmtDuration(avg);
  } else {
    document.getElementById('statAvg').textContent = '—';
  }

  // 表格（倒序）
  const tbody = document.getElementById('adminRecordsBody');
  const empty = document.getElementById('adminEmptyMsg');
  const sorted = [...data.sessions].reverse();

  if (sorted.length === 0) {
    tbody.innerHTML = '';
    empty.style.display = 'block';
    return;
  }
  empty.style.display = 'none';
  tbody.innerHTML = sorted.map(function(s, i) {
    const idx = data.sessions.length - i;
    const isActive = !s.shutdownTime;
    return '<tr>' +
      '<td>' + idx + '</td>' +
      '<td>' + fmtFullTime(s.bootTime) + '</td>' +
      '<td>' + (isActive ? '—' : fmtFullTime(s.shutdownTime)) + '</td>' +
      '<td>' + (isActive ? '—' : fmtDuration(s.duration)) + '</td>' +
      '<td>' + (isActive
        ? '<span class="status-dot active"></span>进行中'
        : '<span class="status-dot closed"></span>已关机') +
      '</td>' +
      '<td><button class="btn-del" data-id="' + s.id + '">删除</button></td>' +
    '</tr>';
  }).join('');
}

function adminDeleteRecord(id) {
  if (!confirm('确定删除这条记录？')) return;
  var data = loadData();
  var idx = data.sessions.findIndex(function(s) { return s.id === id; });
  if (idx === -1) return;

  var session = data.sessions[idx];
  if (session.shutdownTime) {
    data.shutdownCount = Math.max(0, (data.shutdownCount || 0) - 1);
  }
  data.bootCount = Math.max(0, (data.bootCount || 0) - 1);
  data.sessions.splice(idx, 1);

  if (session.id === sessionStorage.getItem('currentSessionId')) {
    sessionStorage.removeItem('currentSessionId');
  }

  // 更新缓存后刷新 UI，不重新调用 initBoot（避免创建新会话）
  window._bootDataCache = data;
  saveData(data);
  updateMainUI(data, data.sessions.find(function(s) { return !s.shutdownTime; }) || data.sessions[data.sessions.length - 1]);
  refreshAdmin();
  refreshMainTable();
  refreshChart();
  renderHeatmap();
}

function clearAll() {
  if (!confirm('确定清空所有记录？此操作不可撤销！')) return;
  var empty = { bootCount: 0, shutdownCount: 0, sessions: [] };
  localStorage.removeItem(STORAGE_KEY);
  localStorage.removeItem(FILE_STORAGE_KEY);
  sessionStorage.removeItem('currentSessionId');
  window._bootDataCache = empty;
  // 也清除文件数据
  if (_isServerAvailable()) {
    fetch(_serverUrl('/api/clear'), { method: 'POST' }).then(function() {
      refreshAdmin();
      refreshMainTable();
      refreshChart();
      renderHeatmap();
      _doInitBoot(empty);
    }).catch(function() {
      refreshAdmin();
      refreshMainTable();
      refreshChart();
      renderHeatmap();
      _doInitBoot(empty);
    });
  } else {
    refreshAdmin();
    refreshMainTable();
    refreshChart();
    renderHeatmap();
    _doInitBoot(empty);
  }
}

function exportCSV() {
  const data = loadData();
  if (data.sessions.length === 0) { alert('暂无记录可导出'); return; }

  var BOM = '\uFEFF';
  var csv = BOM + '序号,开机时间,关机时间,会话时长(秒),状态\n';
  data.sessions.forEach(function(s, i) {
    var dur = s.duration ? (s.duration / 1000) : '';
    var status = s.shutdownTime ? '已关机' : '进行中';
    csv += (i+1) + ',"' + fmtFullTime(s.bootTime) + '","' + fmtFullTime(s.shutdownTime) + '","' + dur + '","' + status + '"\n';
  });

  var blob = new Blob([csv], { type: 'text/csv;charset=utf-8;' });
  var url = URL.createObjectURL(blob);
  var a = document.createElement('a');
  a.href = url;
  a.download = '开机记录_' + new Date().toISOString().slice(0,10) + '.csv';
  a.click();
  URL.revokeObjectURL(url);
}

function exportJSON() {
  const data = loadData();
  if (data.sessions.length === 0) { alert('暂无记录可导出'); return; }

  // 转换成更易读的格式
  var exportData = {
    导出时间: new Date().toLocaleString('zh-CN'),
    总开机次数: data.bootCount,
    总关机次数: data.shutdownCount,
    记录列表: data.sessions.map(function(s, i) {
      return {
        序号: i + 1,
        开机时间: fmtFullTime(s.bootTime),
        关机时间: s.shutdownTime ? fmtFullTime(s.shutdownTime) : '未关机',
        会话时长: s.duration ? fmtDuration(s.duration) : '—',
        状态: s.shutdownTime ? '已关机' : '进行中'
      };
    })
  };

  var jsonStr = JSON.stringify(exportData, null, 2);
  var blob = new Blob([jsonStr], { type: 'application/json;charset=utf-8;' });
  var url = URL.createObjectURL(blob);
  var a = document.createElement('a');
  a.href = url;
  a.download = '开机记录_' + new Date().toISOString().slice(0,10) + '.json';
  a.click();
  URL.revokeObjectURL(url);
}

function exportXLSX() {
  const data = loadData();
  if (data.sessions.length === 0) { alert('暂无记录可导出'); return; }

  // 准备数据：表头 + 数据行
  var headers = ['序号', '开机时间', '关机时间', '会话时长', '状态'];
  var rows = data.sessions.map(function(s, i) {
    return [
      i + 1,
      fmtFullTime(s.bootTime),
      s.shutdownTime ? fmtFullTime(s.shutdownTime) : '未关机',
      s.duration ? fmtDuration(s.duration) : '—',
      s.shutdownTime ? '已关机' : '进行中'
    ];
  });

  // 创建 worksheet
  var wsData = [headers].concat(rows);
  var ws = XLSX.utils.aoa_to_sheet(wsData);

  // 设置列宽
  ws['!cols'] = [
    { wch: 6 },   // 序号
    { wch: 20 },  // 开机时间
    { wch: 20 },  // 关机时间
    { wch: 14 },  // 会话时长
    { wch: 10 }   // 状态
  ];

  // 创建 workbook 并写入
  var wb = XLSX.utils.book_new();
  XLSX.utils.book_append_sheet(wb, ws, '开机记录');

  // 添加统计 sheet
  var statsData = [
    ['统计项', '数值'],
    ['导出时间', new Date().toLocaleString('zh-CN')],
    ['总开机次数', data.bootCount],
    ['总关机次数', data.shutdownCount],
  ];
  var closedSessions = data.sessions.filter(function(s) { return s.duration; });
  if (closedSessions.length > 0) {
    var avg = closedSessions.reduce(function(a, s) { return a + s.duration; }, 0) / closedSessions.length;
    statsData.push(['平均会话时长', fmtDuration(avg)]);
  }
  var wsStats = XLSX.utils.aoa_to_sheet(statsData);
  wsStats['!cols'] = [{ wch: 16 }, { wch: 20 }];
  XLSX.utils.book_append_sheet(wb, wsStats, '统计');

  // 导出文件
  var fileName = '开机记录_' + new Date().toISOString().slice(0,10) + '.xlsx';
  XLSX.writeFile(wb, fileName);
}

// ——— 暗色/浅色切换 ———
function toggleMode() {
  var isLight = document.body.classList.toggle('light-mode');
  var btn = document.getElementById('modeToggle');
  btn.textContent = isLight ? '🌙' : '☀';
  btn.title = isLight ? '切换暗色模式' : '切换浅色模式';
  localStorage.setItem('bootTrackerMode', isLight ? 'light' : 'dark');
}

// 加载保存的模式
(function() {
  var saved = localStorage.getItem('bootTrackerMode');
  if (saved === 'light') {
    document.body.classList.add('light-mode');
    document.getElementById('modeToggle').textContent = '🌙';
    document.getElementById('modeToggle').title = '切换暗色模式';
  }
})();

// ——— 主题切换 ———
function switchTheme(name) {
  document.documentElement.setAttribute('data-theme', name);
  localStorage.setItem('bootTrackerTheme', name);
  // 更新激活状态
  document.querySelectorAll('.theme-dot').forEach(function(d) {
    d.classList.toggle('active', d.getAttribute('data-theme') === name);
  });
  // 刷新图表
  refreshChart();
}
// 加载保存的主题
(function() {
  var saved = localStorage.getItem('bootTrackerTheme') || 'mica';
  switchTheme(saved);
})();

// ——— 快捷键（备用） ———
document.addEventListener('keydown', function(e) {
  if (e.ctrlKey && e.shiftKey && e.key === 'A') {
    e.preventDefault();
    toggleAdmin();
  }
});

// ——— 点击三次开机次数进入管理员 ———
var adminClickCount = 0;
var adminClickTimer = null;
document.getElementById('bootCount').addEventListener('click', function() {
  adminClickCount++;
  if (adminClickTimer) clearTimeout(adminClickTimer);
  adminClickTimer = setTimeout(function() { adminClickCount = 0; }, 1500);
  if (adminClickCount >= 3) {
    adminClickCount = 0;
    clearTimeout(adminClickTimer);
    toggleAdmin();
  }
});
document.getElementById('bootCount').style.cursor = 'pointer';
document.getElementById('bootCount').title = '';

// ——— 日期筛选 ———
function getFilteredSessions() {
  const data = loadData();
  const from = document.getElementById('filterFrom').value;
  const to = document.getElementById('filterTo').value;
  return data.sessions.filter(function(s) {
    const d = s.bootTime.slice(0, 10);
    if (from && d < from) return false;
    if (to && d > to) return false;
    return true;
  });
}

function clearFilter() {
  document.getElementById('filterFrom').value = '';
  document.getElementById('filterTo').value = '';
  refreshMainTable();
  refreshChart();
  renderHeatmap();
}

// ——— 图表（纯 SVG，无外部依赖） ———
var currentChartType = 'line';

function switchChart(type) {
  currentChartType = type;
  document.querySelectorAll('.chart-tab').forEach(function(b) {
    b.classList.toggle('active', (type === 'bar' && b.textContent === '柱状图') || (type === 'line' && b.textContent === '折线图'));
  });
  refreshChart();
}

function refreshChart() {
  var svg = document.getElementById('chartSVG');
  if (!svg) return;
  var sessions = getFilteredSessions();
  if (sessions.length === 0) {
    svg.innerHTML = '<text x="400" y="150" text-anchor="middle" fill="#64748b" font-family="Inter,sans-serif" font-size="13">暂无数据</text>';
    return;
  }

  // 读取主题色
  var style = getComputedStyle(document.documentElement);
  var accent = style.getPropertyValue('--accent').trim() || '#8b5cf6';
  var accentRgb = style.getPropertyValue('--accent-rgb').trim() || '139,92,246';
  // 云母主题使用柔和的图表色
  var currentTheme = document.documentElement.getAttribute('data-theme');
  if (currentTheme === 'mica') {
    accent = '#94a3b8';
    accentRgb = '148,163,184';
  }

  // 按日期汇总时长（分钟）
  var now = new Date();
  var byDate = {};
  sessions.forEach(function(s) {
    var day = s.bootTime.slice(0, 10);
    if (!byDate[day]) byDate[day] = 0;
    var dur = s.duration || (now - new Date(s.bootTime));
    byDate[day] += dur;
  });
  var dates = Object.keys(byDate).sort();
  var values = dates.map(function(d) { return Math.round(byDate[d] / 60000); });

  // SVG 视口
  var W = 800, H = 300;
  var padL = 50, padR = 20, padT = 20, padB = 40;
  var plotW = W - padL - padR;
  var plotH = H - padT - padB;

  var maxVal = Math.max.apply(null, values) || 1;
  maxVal = Math.ceil(maxVal * 1.15); // 留顶部空间

  var cols = dates.length;
  var barW = Math.min(40, Math.max(6, (plotW / cols) * 0.7));
  var gap = plotW / cols;

  var svgHTML = '';
  // 网格线
  var gridLines = 5;
  for (var i = 0; i <= gridLines; i++) {
    var y = padT + (plotH / gridLines) * i;
    var val = Math.round(maxVal - (maxVal / gridLines) * i);
    svgHTML += '<line x1="' + padL + '" y1="' + y + '" x2="' + (padL + plotW) + '" y2="' + y + '" stroke="rgba(' + accentRgb + ',0.08)" stroke-width="1"/>';
    svgHTML += '<text x="' + (padL - 8) + '" y="' + (y + 4) + '" text-anchor="end" fill="#64748b" font-family="Inter,sans-serif" font-size="10">' + val + '</text>';
  }

  // 数据点/柱子
  var points = [];
  var pathD = '';
  values.forEach(function(v, idx) {
    var cx = padL + gap * idx + gap / 2;
    var cy = padT + plotH - (v / maxVal) * plotH;
    points.push({ x: cx, y: cy, v: v, date: dates[idx] });

    if (currentChartType === 'bar') {
      svgHTML += '<rect x="' + (cx - barW/2) + '" y="' + cy + '" width="' + barW + '" height="' + (padT + plotH - cy) + '" rx="3" fill="rgba(' + accentRgb + ',0.5)"/>';
    }

    if (idx === 0) pathD = 'M' + cx + ',' + cy;
    else pathD += ' L' + cx + ',' + cy;
  });

  // 折线 / 面积
  if (currentChartType === 'line') {
    var areaD = pathD + ' L' + (padL + plotW) + ',' + (padT + plotH) + ' L' + padL + ',' + (padT + plotH) + ' Z';
    svgHTML += '<path d="' + areaD + '" fill="rgba(' + accentRgb + ',0.08)"/>';
    svgHTML += '<path d="' + pathD + '" fill="none" stroke="' + accent + '" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"/>';
  }

  // 数据点圆点
  points.forEach(function(p) {
    svgHTML += '<circle cx="' + p.x + '" cy="' + p.y + '" r="' + (currentChartType === 'line' ? 3.5 : 0) + '" fill="' + accent + '"/>';
    svgHTML += '<title>' + p.date + ': ' + p.v + ' 分钟</title>';
  });

  // X 轴标签（简略：只标每 N 个）
  var step = Math.max(1, Math.floor(cols / 8));
  dates.forEach(function(d, idx) {
    if (idx % step === 0 || idx === dates.length - 1) {
      var cx = padL + gap * idx + gap / 2;
      var label = d.slice(5); // MM-DD
      svgHTML += '<text x="' + cx + '" y="' + (H - 8) + '" text-anchor="middle" fill="#64748b" font-family="Inter,sans-serif" font-size="9">' + label + '</text>';
    }
  });

  // Y 轴标签
  svgHTML += '<text x="4" y="' + (padT + plotH/2) + '" text-anchor="middle" fill="#64748b" font-family="Inter,sans-serif" font-size="9" transform="rotate(-90,4,' + (padT + plotH/2) + ')">分钟</text>';

  svg.innerHTML = svgHTML;
}

function refreshMainTable() {
  const sessions = getFilteredSessions();
  const tbody = document.getElementById('mainRecordsBody');
  const empty = document.getElementById('mainEmptyMsg');
  const sorted = [].concat(sessions).reverse();

  if (sorted.length === 0) {
    tbody.innerHTML = '';
    empty.style.display = 'block';
    return;
  }
  empty.style.display = 'none';

  var total = (loadData().sessions || []).length;
  tbody.innerHTML = sorted.map(function(s, i) {
    var idx = total - (sessions.length - 1 - i);
    var isActive = !s.shutdownTime;
    return '<tr>' +
      '<td>' + idx + '</td>' +
      '<td>' + fmtTime(s.bootTime) + '</td>' +
      '<td>' + (isActive ? '—' : fmtTime(s.shutdownTime)) + '</td>' +
      '<td>' + (isActive ? '—' : fmtDuration(s.duration)) + '</td>' +
      '<td>' + (isActive
        ? '<span class="status-dot active"></span>进行中'
        : '<span class="status-dot closed"></span>已关机') + '</td>' +
    '</tr>';
  }).join('');
  renderHeatmap();
}


// ——— 热度图渲染 ———
function getHeatColor(count, max) {
  var isMica = document.documentElement.getAttribute('data-theme') === 'mica';
  if (count === 0) return isMica ? 'rgba(255,255,255,0.03)' : 'rgba(30,41,59,0.5)';
  var r = count / max;
  if (isMica) {
    // 云母：灰阶热力
    if (r <= 0.25) return 'rgba(148,163,184,0.25)';
    if (r <= 0.5) return 'rgba(148,163,184,0.4)';
    if (r <= 0.75) return 'rgba(148,163,184,0.6)';
    return 'rgba(148,163,184,0.8)';
  }
  if (r <= 0.25) return 'rgba(88,28,135,0.7)';
  if (r <= 0.5) return 'rgba(124,58,237,0.75)';
  if (r <= 0.75) return 'rgba(139,92,246,0.85)';
  return 'rgba(167,139,250,0.95)';
}

function renderHeatmap() {
  var container = document.getElementById('heatmapContainer');
  var sessions = getFilteredSessions();
  if (!sessions || sessions.length === 0) {
    container.innerHTML = '<div style="text-align:center;color:#888;padding:40px;font-size:0.85rem;">暂无数据</div>';
    return;
  }

  // 统计每天开机次数
  var byDay = {};
  sessions.forEach(function(s) {
    var day = s.bootTime.slice(0, 10);
    byDay[day] = (byDay[day] || 0) + 1;
  });

  var dates = Object.keys(byDay).sort();
  if (dates.length === 0) {
    container.innerHTML = '<div style="text-align:center;color:#888;padding:40px;font-size:0.85rem;">暂无数据</div>';
    return;
  }

  var maxCount = 0;
  for (var d in byDay) { if (byDay[d] > maxCount) maxCount = byDay[d]; }
  if (maxCount === 0) maxCount = 1;

  var months = ['1月','2月','3月','4月','5月','6月','7月','8月','9月','10月','11月','12月'];
  var dayNames = ['日','一','二','三','四','五','六'];
  var CELL = 14, GAP = 3;

  // 日期范围：至少显示最近 12 周
  var firstDate = new Date(dates[0] + 'T00:00:00');
  var lastDate = new Date(dates[dates.length-1] + 'T00:00:00');
  var start = new Date(firstDate.getTime() - firstDate.getDay() * 86400000);
  var end = new Date(lastDate.getTime() + (6 - lastDate.getDay()) * 86400000);
  
  // 如果不足 12 周，扩展到 12 周
  var minStart = new Date();
  minStart.setDate(minStart.getDate() - 12 * 7);
  minStart.setDate(minStart.getDate() - minStart.getDay()); // 对齐到周日
  if (start > minStart) start = minStart;

  // 构建所有格子（行优顺序：按日期排列）
  var cells = [];
  var c = new Date(start);
  while (c <= end) {
    var key = c.toISOString().slice(0, 10);
    var count = byDay[key] || 0;
    cells.push({ date: new Date(c), count: count, key: key });
    c.setDate(c.getDate() + 1);
  }

  // 计算周数和月份标签
  var weekCount = Math.ceil(cells.length / 7);
  var monthSections = [];
  for (var w = 0; w < weekCount; w++) {
    var weekDate = cells[w * 7].date;
    monthSections.push(weekDate.getMonth());
  }

  var lastM = -1;
  var monthRow = '';
  for (var i = 0; i < weekCount; i++) {
    if (monthSections[i] !== lastM) {
      monthRow += '<div style="width:' + (CELL + GAP) + 'px;flex-shrink:0;font-size:0.65rem;color:#64748b;">' + months[monthSections[i]] + '</div>';
      lastM = monthSections[i];
    } else {
      monthRow += '<div style="width:' + (CELL + GAP) + 'px;flex-shrink:0;"></div>';
    }
  }

  // 构建周列（每列 7 个格子）
  var weekCols = '';
  for (var col = 0; col < weekCount; col++) {
    weekCols += '<div style="display:flex;flex-direction:column;gap:' + GAP + 'px;">';
    for (var row = 0; row < 7; row++) {
      var idx = col * 7 + row;
      if (idx >= cells.length) break;
      var cell = cells[idx];
      var bg = getHeatColor(cell.count, maxCount);
      var tip = cell.count > 0 ? cell.key + '  ' + cell.count + '次开机' : cell.key + '  无记录';
      weekCols += '<div class="hm-cell-el" style="background:' + bg + ';width:' + CELL + 'px;height:' + CELL + 'px;" data-tip="' + tip + '" title="' + tip + '"></div>';
    }
    weekCols += '</div>';
  }

  container.innerHTML = 
    '<div style="overflow-x:auto;padding:8px 0;">' +
      '<div style="display:flex;flex-direction:column;gap:4px;">' +
        '<div style="display:flex;padding-left:30px;">' + monthRow + '</div>' +
        '<div style="display:flex;gap:' + GAP + 'px;">' +
          '<div style="display:flex;flex-direction:column;gap:' + GAP + 'px;font-size:0.65rem;color:#64748b;width:26px;flex-shrink:0;padding-top:0;">' +
            dayNames.map(function(d) { return '<div style="height:' + CELL + 'px;line-height:' + CELL + 'px;text-align:right;padding-right:4px;">' + d + '</div>'; }).join('') +
          '</div>' +
          '<div style="display:flex;gap:' + GAP + 'px;">' + weekCols + '</div>' +
        '</div>' +
      '</div>' +
    '</div>';
}

// ——— 管理员表格事件委托 ———
document.getElementById('adminRecordsBody').addEventListener('click', function(e) {
  var btn = e.target.closest('.btn-del');
  if (!btn) return;
  var id = btn.getAttribute('data-id');
  if (id) adminDeleteRecord(id);
});

// ——— 启动 ———
_serverPort = _detectServerPort();
if (_serverPort) {
  // 桌面模式：等待服务器就绪后加载数据
  document.getElementById('currentBootTime').textContent = '连接中...';
  document.getElementById('currentStatus').innerHTML = '等待后台服务';
  document.getElementById('runningTime').textContent = '...';
  loadDataAsync(function(data) {
    window._bootDataCache = data;
    _doInitBoot(data);
  });
} else {
  // 纯浏览器模式
  setTimeout(function() { _doInitBoot(loadData()); }, 200);
}
