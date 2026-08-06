const STORAGE_KEY = 'bootTrackerData';
const FILE_STORAGE_KEY = 'bootTrackerData_file';

/* ——— 文件存储桥接（通过 HTTP 接口与 Python 通信）——— */
var _serverPort = null;
var _serverChecked = false;
var _serverOnline = false;

function _detectServerPort() {
  // 从 URL hash 读取端口：index.html#port=18792
  var hash = window.location.hash;
  var m = hash.match(/port=(\d+)/);
  if (m) return parseInt(m[1], 10);
  // 远程访问时直接从 URL 读取端口
  if (window.location.port) return parseInt(window.location.port, 10);
  // 回退到默认端口
  return 18792;
}

function _isServerAvailable() {
  return _serverPort !== null;
}

function _isServerOnline() {
  return _serverOnline;
}

function _serverUrl(path) {
  // pyWebview 使用 file:// 协议，需要构造完整的 HTTP URL
  if (window.location.protocol === 'file:' && _serverPort) {
    return 'http://127.0.0.1:' + _serverPort + path;
  }
  // 浏览器访问（http:// 或 https://）使用相对路径
  return path;
}

// 异步检测服务器是否真正在线（通过 ping）
function _checkServerOnline(callback) {
  if (!_isServerAvailable()) {
    _serverOnline = false;
    _serverChecked = true;
    callback(false);
    return;
  }
  // 设置超时 3 秒
  var timeoutId = setTimeout(function() {
    _serverOnline = false;
    _serverChecked = true;
    callback(false);
  }, 3000);

  fetch(_serverUrl('/api/ping'))
    .then(function(r) { return r.json(); })
    .then(function(data) {
      clearTimeout(timeoutId);
      _serverOnline = !!(data && data.ok);
      _serverChecked = true;
      callback(_serverOnline);
    })
    .catch(function() {
      clearTimeout(timeoutId);
      _serverOnline = false;
      _serverChecked = true;
      callback(false);
    });
}

/* ——— 数据加载（优先文件服务器，回退 localStorage）——— */
function loadData() {
  // 同步返回：返回缓存或 localStorage
  if (window._bootDataCache) {
    try { return JSON.parse(JSON.stringify(window._bootDataCache)); } catch(e) {}
    // 深度拷贝失败时回退到浅拷贝
    return {
      bootCount: window._bootDataCache.bootCount || 0,
      shutdownCount: window._bootDataCache.shutdownCount || 0,
      sessions: (window._bootDataCache.sessions || []).slice()
    };
  }

  var raw = localStorage.getItem(STORAGE_KEY);
  if (raw) {
    try { var d = JSON.parse(raw); window._bootDataCache = d; return JSON.parse(JSON.stringify(d)); } catch(e) {}
  }
  return { bootCount: 0, shutdownCount: 0, sessions: [] };
}

/* 异步加载数据（先验证服务器在线，再用文件服务器或 localStorage） */
function loadDataAsync(callback, retries) {
  // 先用本地缓存立即渲染（避免空白等待）
  var localData = loadData();
  if (localData.sessions && localData.sessions.length > 0) {
    callback(localData);
  }

  if (!_isServerAvailable()) {
    // 没有服务器配置（纯浏览器模式）
    if (!localData.sessions || localData.sessions.length === 0) {
      callback(localData);
    }
    return;
  }

  // 检测服务器是否真正在线
  _checkServerOnline(function(online) {
    if (!online) {
      // 服务器离线，保留已有本地数据
      if (!localData.sessions || localData.sessions.length === 0) {
        // 没有任何数据时显示离线状态
        _showOfflineState();
        callback(localData);
      }
      return;
    }

    // 服务器在线，拉取最新数据
    fetch(_serverUrl('/api/data'))
      .then(function(r) { return r.json(); })
      .then(function(data) {
        if (data && typeof data === 'object' && !data.error) {
          window._bootDataCache = data;
          window._serverReady = true;
          try { localStorage.setItem(STORAGE_KEY, JSON.stringify(data)); localStorage.setItem(FILE_STORAGE_KEY, '1'); } catch(e) {}
          // 清除加载状态
          _clearLoadingState();
          callback(data);
        }
      })
      .catch(function() {
        // 数据拉取失败，保留已有数据
        if (!localData.sessions || localData.sessions.length === 0) {
          callback(localData);
        }
      });
  });
}

function _showOfflineState() {
  // 显示离线状态提示
  var statusEl = document.getElementById('currentStatus');
  var bootTimeEl = document.getElementById('currentBootTime');
  var runTimeEl = document.getElementById('runningTime');
  if (statusEl) statusEl.innerHTML = '<span class="status-dot closed"></span>未连接服务';
  if (bootTimeEl) bootTimeEl.textContent = '—';
  if (runTimeEl) runTimeEl.textContent = '—';
}

function _clearLoadingState() {
  // 清除加载状态（让 updateMainUI 重新填充正确状态）
  // 这个函数由 callback 中的 _doInitBoot 自动处理
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

// ISO 时间字符串 → 本地日期字符串 "2026-06-20"
function isoToLocalDate(iso) {
  if (!iso) return '';
  var d = new Date(iso);
  var y = d.getFullYear();
  var m = ('0' + (d.getMonth() + 1)).slice(-2);
  var day = ('0' + d.getDate()).slice(-2);
  return y + '-' + m + '-' + day;
}

// 本地日期字符串 "2026-06-20"
function getLocalToday() {
  var d = new Date();
  var y = d.getFullYear();
  var m = ('0' + (d.getMonth() + 1)).slice(-2);
  var day = ('0' + d.getDate()).slice(-2);
  return y + '-' + m + '-' + day;
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

  // 没有活跃会话：仅展示已关机状态，不自动创建
  updateMainUI(data, null);
  refreshMainTable();
  refreshChart();
  renderHeatmap();
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
  // 计算今日开机次数
  var today = getLocalToday();
  var todayCount = data.sessions.filter(function(s) { return isoToLocalDate(s.bootTime) === today; }).length;
  var el = document.getElementById('bootCount');
  if (el) el.textContent = todayCount || 0;
  var tc = document.getElementById('totalCount');
  if (tc) tc.textContent = data.bootCount || 0;
  var sc = document.getElementById('shutdownCount');
  if (sc) sc.textContent = data.shutdownCount || 0;
  document.getElementById('currentBootTime').textContent = currentSession ? fmtFullTime(currentSession.bootTime) : '—';

  const btn = document.getElementById('btnShutdown');
  const rtEl = document.getElementById('runningTime');
  if (!currentSession || currentSession.shutdownTime) {
    document.getElementById('currentStatus').innerHTML = '<span class="status-dot closed"></span>已关机';
    btn.disabled = true;
    btn.textContent = '已记录关机';
    // 无活跃会话：停止计时器，重置运行时长
    if (timerInterval) { clearInterval(timerInterval); timerInterval = null; }
    rtEl.textContent = '—';
  } else {
    document.getElementById('currentStatus').innerHTML = '<span class="status-dot active"></span>运行中';
    btn.disabled = false;
    btn.textContent = '记录关机';
    // 有活跃会话：确保计时器在跑（避免重复 startTimer）
    if (!timerInterval) startTimer(currentSession.bootTime);
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

// ——— 按天分组工具函数 ———
function _groupByDate(sessions) {
  var groups = {};
  var order = [];
  sessions.forEach(function(s) {
    var day = isoToLocalDate(s.bootTime);
    if (!groups[day]) { groups[day] = []; order.push(day); }
    groups[day].push(s);
  });
  return { groups: groups, order: order };
}

function _dayTotalDuration(sessions) {
  var now = Date.now();
  var total = 0;
  sessions.forEach(function(s) {
    var dur = s.duration || (now - new Date(s.bootTime).getTime());
    total += dur;
  });
  return total;
}

function _dayLabel(dateStr) {
  var parts = dateStr.split('-');
  var d = new Date(parseInt(parts[0]), parseInt(parts[1]) - 1, parseInt(parts[2]));
  var weekdays = ['周日','周一','周二','周三','周四','周五','周六'];
  var now = new Date();
  var today = new Date(now.getFullYear(), now.getMonth(), now.getDate());
  var yesterday = new Date(today); yesterday.setDate(yesterday.getDate() - 1);
  var target = new Date(d.getFullYear(), d.getMonth(), d.getDate());
  var tag = '';
  if (target.getTime() === today.getTime()) tag = ' · 今天';
  else if (target.getTime() === yesterday.getTime()) tag = ' · 昨天';
  var m = d.getMonth() + 1, day = d.getDate();
  var monthNames = ['1月','2月','3月','4月','5月','6月','7月','8月','9月','10月','11月','12月'];
  return monthNames[m-1] + day + '日 ' + weekdays[d.getDay()] + tag;
}

// 折叠状态存储
var _collapsedDays = {};
try {
  var saved = sessionStorage.getItem('bootTrackerCollapsed');
  if (saved) _collapsedDays = JSON.parse(saved);
} catch(e) {}

function toggleDayGroup(dateStr, tableType) {
  // 首次 toggle 时（key 不存在），默认折叠→展开
  if (!_collapsedDays.hasOwnProperty(dateStr)) {
    _collapsedDays[dateStr] = true;
  }
  _collapsedDays[dateStr] = !_collapsedDays[dateStr];
  try { sessionStorage.setItem('bootTrackerCollapsed', JSON.stringify(_collapsedDays)); } catch(e) {}
  var tbody = document.getElementById(tableType === 'admin' ? 'adminRecordsBody' : 'mainRecordsBody');
  // CSS class 用的是去掉 tableType 前缀的原始日期
  var rawDate = dateStr.replace(/^(admin_)/, '');
  var cls = rawDate.replace(/-/g, '_');
  var rows = tbody.querySelectorAll('.day-group-' + cls);
  for (var i = 0; i < rows.length; i++) {
    rows[i].style.display = _collapsedDays[dateStr] ? 'none' : '';
  }
  var headers = tbody.querySelectorAll('.day-header-' + cls);
  for (var i = 0; i < headers.length; i++) {
    headers[i].classList.toggle('collapsed', !!_collapsedDays[dateStr]);
    var arrow = headers[i].querySelector('.day-arrow');
    if (arrow) arrow.innerHTML = _collapsedDays[dateStr] ? '&#9654;' : '&#9660;';
  }
}

// ——— 管理员面板 ———
function toggleAdmin() {
  const overlay = document.getElementById('adminOverlay');
  overlay.classList.toggle('active');
  if (overlay.classList.contains('active')) {
    refreshAdmin();
    refreshTrash();
  }
}

function switchAdminTab(tab, btn) {
  // 切换标签高亮
  document.querySelectorAll('.admin-tab').forEach(function(t) { t.classList.remove('active'); });
  btn.classList.add('active');
  // 切换内容
  document.querySelectorAll('.admin-tab-content').forEach(function(c) { c.classList.remove('active'); });
  document.getElementById('tab' + tab.charAt(0).toUpperCase() + tab.slice(1)).classList.add('active');
  // 切换到回收站时刷新
  if (tab === 'trash') refreshTrash();
  // 切换到备份还原时加载列表
  if (tab === 'backup') loadBackupList();
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

  // 表格（倒序，按天分组）
  var tbody = document.getElementById('adminRecordsBody');
  var empty = document.getElementById('adminEmptyMsg');
  var sorted = [].concat(data.sessions).reverse();

  if (sorted.length === 0) {
    tbody.innerHTML = '';
    empty.style.display = 'block';
    return;
  }
  empty.style.display = 'none';

  var dayInfo = _groupByDate(sorted);
  var globalIdx = 0;
  var html = '';

  dayInfo.order.reverse().forEach(function(dateStr) {
    var group = dayInfo.groups[dateStr];
    var collapsed = _collapsedDays.hasOwnProperty('admin_' + dateStr) ? !!_collapsedDays['admin_' + dateStr] : true;
    var cls = 'day-header-' + dateStr.replace(/-/g, '_');
    var dur = _dayTotalDuration(group);
    var hasActive = group.some(function(s) { return !s.shutdownTime; });

    html += '<tr class="day-header ' + cls + (collapsed ? ' collapsed' : '') + '" onclick="toggleDayGroup(\'admin_' + dateStr + '\', \'admin\')">' +
      '<td colspan="7">' +
        '<span class="day-arrow">' + (collapsed ? '&#9654;' : '&#9660;') + '</span>' +
        '<span class="day-label">' + _dayLabel(dateStr) + '</span>' +
        '<span class="day-count">' + group.length + '条记录</span>' +
        '<span class="day-duration">' + fmtDuration(dur) + '</span>' +
        (hasActive ? '<span class="status-dot active day-active-dot"></span>' : '') +
      '</td>' +
    '</tr>';

    group.forEach(function(s) {
      globalIdx++;
      var idx = data.sessions.length - globalIdx + 1;
      var isActive = !s.shutdownTime;
      var gcls = 'day-group-' + dateStr.replace(/-/g, '_');
      html += '<tr class="session-row ' + gcls + '"' + (collapsed ? ' style="display:none"' : '') + '>' +
        '<td class="td-check"><input type="checkbox" data-id="' + s.id + '" class="admin-check"></td>' +
        '<td>' + idx + '</td>' +
        '<td>' + fmtFullTime(s.bootTime) + '</td>' +
        '<td>' + (isActive ? '\u2014' : fmtFullTime(s.shutdownTime)) + '</td>' +
        '<td>' + (isActive ? '\u2014' : fmtDuration(s.duration)) + '</td>' +
        '<td>' + (isActive
          ? '<span class="status-dot active"></span>\u8fdb\u884c\u4e2d'
          : '<span class="status-dot closed"></span>\u5df2\u5173\u673a') +
        '</td>' +
        '<td class="admin-actions-cell"><button class="btn-edit" data-id="' + s.id + '">\u7f16\u8f91</button><button class="btn-del" data-id="' + s.id + '">\u5220\u9664</button></td>' +
      '</tr>';
    });
  });

  tbody.innerHTML = html;
}

function adminDeleteRecord(id) {
  if (!confirm('确定删除这条记录？将移入回收站，可恢复。')) return;
  var data = loadData();
  var idx = data.sessions.findIndex(function(s) { return s.id === id; });
  if (idx === -1) return;

  var session = data.sessions.splice(idx, 1)[0];

  if (session.shutdownTime) {
    data.shutdownCount = Math.max(0, (data.shutdownCount || 0) - 1);
  }
  data.bootCount = Math.max(0, (data.bootCount || 0) - 1);

  if (session.id === sessionStorage.getItem('currentSessionId')) {
    sessionStorage.removeItem('currentSessionId');
  }

  // 保存到回收站
  var trashRaw = localStorage.getItem('bootTrackerTrash');
  var trash = trashRaw ? JSON.parse(trashRaw) : { sessions: [] };
  trash.sessions.push(session);
  localStorage.setItem('bootTrackerTrash', JSON.stringify(trash));

  // 更新本地缓存和 UI（乐观更新）
  window._bootDataCache = data;
  try { localStorage.setItem(STORAGE_KEY, JSON.stringify(data)); } catch(e) {}
  updateMainUI(data, data.sessions.find(function(s) { return !s.shutdownTime; }) || data.sessions[data.sessions.length - 1]);
  refreshAdmin();
  refreshMainTable();
  refreshChart();
  renderHeatmap();

  // 如果有服务器，同步到服务端（DELETE 确保服务端数据一致）
  if (_isServerAvailable() && window._serverReady) {
    fetch(_serverUrl('/api/sessions/' + id), { method: 'DELETE' })
      .then(function(r) { return r.json(); })
      .then(function(res) {
        if (!res.ok) {
          // 服务端删除失败，重新加载数据恢复
          loadDataAsync(function(serverData) {
            window._bootDataCache = serverData;
            try { localStorage.setItem(STORAGE_KEY, JSON.stringify(serverData)); } catch(e) {}
            var active = serverData.sessions.find(function(s) { return !s.shutdownTime; });
            updateMainUI(serverData, active || null);
            refreshAdmin();
            refreshMainTable();
            refreshChart();
            renderHeatmap();
          });
        }
      })
      .catch(function() {
        // 网络错误，重新加载数据
        loadDataAsync(function(serverData) {
          window._bootDataCache = serverData;
          try { localStorage.setItem(STORAGE_KEY, JSON.stringify(serverData)); } catch(e) {}
          var active = serverData.sessions.find(function(s) { return !s.shutdownTime; });
          updateMainUI(serverData, active || null);
          refreshAdmin();
          refreshMainTable();
          refreshChart();
          renderHeatmap();
        });
      });
    // 同步回收站到服务端
    fetch(_serverUrl('/api/trash'), {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(trash)
    }).catch(function() {});
  }
}

function clearAll() {
  if (!confirm('确定清空所有记录？此操作不可撤销！')) return;
  localStorage.removeItem(STORAGE_KEY);
  localStorage.removeItem(FILE_STORAGE_KEY);
  sessionStorage.removeItem('currentSessionId');

  if (_isServerAvailable()) {
    fetch(_serverUrl('/api/clear'), { method: 'POST' })
      .then(function() {
        // 清空成功，从服务端重新加载（确保一致性）
        loadDataAsync(function(serverData) {
          window._bootDataCache = serverData;
          try { localStorage.setItem(STORAGE_KEY, JSON.stringify(serverData)); } catch(e) {}
          _doInitBoot(serverData);
        });
      })
      .catch(function() {
        // 请求失败，从服务端重新加载
        loadDataAsync(function(serverData) {
          window._bootDataCache = serverData;
          try { localStorage.setItem(STORAGE_KEY, JSON.stringify(serverData)); } catch(e) {}
          _doInitBoot(serverData);
        });
      });
  } else {
    var empty = { bootCount: 0, shutdownCount: 0, sessions: [] };
    window._bootDataCache = empty;
    _doInitBoot(empty);
  }
}

// ——— 回收站 ———
function refreshTrash() {
  var trashRaw = localStorage.getItem('bootTrackerTrash');
  var trash = trashRaw ? JSON.parse(trashRaw) : { sessions: [] };

  // 如果有服务器，尝试从服务器加载更完整的回收站数据
  if (_isServerAvailable() && window._serverReady) {
    fetch(_serverUrl('/api/trash'))
      .then(function(r) { return r.json(); })
      .then(function(serverTrash) {
        if (serverTrash && serverTrash.sessions) {
          trash = serverTrash;
          localStorage.setItem('bootTrackerTrash', JSON.stringify(trash));
        }
        _renderTrashTable(trash);
      })
      .catch(function() {
        _renderTrashTable(trash);
      });
  } else {
    _renderTrashTable(trash);
  }
}

function _renderTrashTable(trash) {
  var tbody = document.getElementById('trashRecordsBody');
  var empty = document.getElementById('trashEmptyMsg');
  var sorted = [].concat(trash.sessions || []).reverse();

  if (sorted.length === 0) {
    tbody.innerHTML = '';
    empty.style.display = 'block';
    return;
  }
  empty.style.display = 'none';
  tbody.innerHTML = sorted.map(function(s, i) {
    var idx = (trash.sessions || []).length - i;
    var isActive = !s.shutdownTime;
    return '<tr>' +
      '<td>' + idx + '</td>' +
      '<td>' + fmtFullTime(s.bootTime) + '</td>' +
      '<td>' + (isActive ? '—' : fmtFullTime(s.shutdownTime)) + '</td>' +
      '<td>' + (isActive ? '—' : fmtDuration(s.duration)) + '</td>' +
      '<td><button class="btn-restore" data-id="' + s.id + '">恢复</button></td>' +
    '</tr>';
  }).join('');
}

function trashRestore(id) {
  if (!confirm('确定恢复这条记录？')) return;
  if (_isServerAvailable() && window._serverReady) {
    fetch(_serverUrl('/api/trash/restore'), {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ id: id })
    }).then(function(r) { return r.json(); })
      .then(function(res) {
        if (res.ok) {
          // 从本地回收站移除
          var trashRaw = localStorage.getItem('bootTrackerTrash');
          var trash = trashRaw ? JSON.parse(trashRaw) : { sessions: [] };
          trash.sessions = trash.sessions.filter(function(s) { return s.id !== id; });
          localStorage.setItem('bootTrackerTrash', JSON.stringify(trash));
          // 重新加载主数据
          loadDataAsync(function(data) {
            window._bootDataCache = data;
            updateMainUI(data, data.sessions.find(function(s) { return !s.shutdownTime; }) || null);
            refreshAdmin();
            refreshMainTable();
            refreshChart();
            renderHeatmap();
            refreshTrash();
          });
        }
      }).catch(function() { alert('恢复失败，请重试'); });
  } else {
    // 纯浏览器模式恢复
    var trashRaw = localStorage.getItem('bootTrackerTrash');
    var trash = trashRaw ? JSON.parse(trashRaw) : { sessions: [] };
    var idx = trash.sessions.findIndex(function(s) { return s.id === id; });
    if (idx === -1) return;
    var session = trash.sessions.splice(idx, 1)[0];
    localStorage.setItem('bootTrackerTrash', JSON.stringify(trash));

    var data = loadData();
    data.sessions.push(session);
    data.sessions.sort(function(a, b) { return (a.bootTime || '').localeCompare(b.bootTime || ''); });
    data.bootCount = data.sessions.length;
    data.shutdownCount = data.sessions.filter(function(s) { return s.shutdownTime; }).length;
    saveData(data);

    updateMainUI(data, data.sessions.find(function(s) { return !s.shutdownTime; }) || null);
    refreshAdmin();
    refreshMainTable();
    refreshChart();
    renderHeatmap();
    refreshTrash();
  }
}

function clearTrash() {
  if (!confirm('确定清空回收站？此操作不可撤销！')) return;
  localStorage.removeItem('bootTrackerTrash');
  if (_isServerAvailable() && window._serverReady) {
    fetch(_serverUrl('/api/trash/clear'), { method: 'POST' }).catch(function() {});
  }
  refreshTrash();
}

// 回收站表格事件委托
try {
  var trashBody = document.getElementById('trashRecordsBody');
  if (trashBody) {
    trashBody.addEventListener('click', function(e) {
      var btn = e.target.closest('.btn-restore');
      if (!btn) return;
      var id = btn.getAttribute('data-id');
      if (id) trashRestore(id);
    });
  }
} catch(e) {}

// ——— 备份还原 ———
function loadBackupList() {
  var wrap = document.getElementById('backupListWrap');
  var empty = document.getElementById('backupEmptyMsg');
  var msg = document.getElementById('backupFormMsg');

  if (_isServerAvailable() && window._serverReady) {
    fetch(_serverUrl('/api/backups'))
      .then(function(r) { return r.json(); })
      .then(function(res) {
        msg.textContent = '';
        renderBackupList(res.backups || []);
      })
      .catch(function() {
        msg.textContent = '加载失败';
        msg.className = 'form-msg error';
      });
  } else {
    // 纯浏览器模式无备份功能
    wrap.innerHTML = '<div class="admin-empty">仅桌面模式支持备份还原</div>';
  }
}

function renderBackupList(files) {
  var wrap = document.getElementById('backupListWrap');
  var empty = document.getElementById('backupEmptyMsg');

  if (!files || files.length === 0) {
    wrap.innerHTML = '<div class="admin-empty">暂无备份文件</div>';
    return;
  }

  // 从文件名解析时间：boot-data-20260614-134515.json → 2026-06-14 13:45:15
  function parseBackupTime(fname) {
    var m = fname.match(/boot-data-(\d{4})(\d{2})(\d{2})-(\d{2})(\d{2})(\d{2})/);
    if (!m) return fname;
    return m[1] + '-' + m[2] + '-' + m[3] + ' ' + m[4] + ':' + m[5] + ':' + m[6];
  }

  var html = '<table class="admin-table"><thead><tr><th>#</th><th>备份时间</th><th>文件名</th><th>操作</th></tr></thead><tbody>';
  files.forEach(function(f, i) {
    var encoded = encodeURIComponent(f);
    html += '<tr>' +
      '<td>' + (files.length - i) + '</td>' +
      '<td>' + parseBackupTime(f) + '</td>' +
      '<td style="font-family:monospace;font-size:0.78rem;color:#94a3b8;">' + f + '</td>' +
      '<td style="white-space:nowrap;">' +
        '<button class="btn-sm success" onclick="restoreBackup(\'' + f + '\')">还原</button> ' +
        '<button class="btn-sm danger btn-del-backup" onclick="deleteBackup(\'' + encoded + '\', this)">删除</button>' +
      '</td>' +
      '</tr>';
  });
  html += '</tbody></table>';
  wrap.innerHTML = html;
}

function restoreBackup(filename) {
  if (!confirm('确定从以下备份还原数据？\n\n' + filename + '\n\n当前数据会自动备份后再被覆盖。')) return;

  var msg = document.getElementById('backupFormMsg');
  msg.textContent = '正在还原...';
  msg.className = 'form-msg';

  fetch(_serverUrl('/api/backup/restore'), {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ filename: filename })
  }).then(function(r) { return r.json(); })
    .then(function(res) {
      if (res.ok) {
        msg.textContent = '还原成功，正在刷新...';
        msg.className = 'form-msg success';
        // 重新加载数据并刷新所有 UI
        loadDataAsync(function(freshData) {
          window._bootDataCache = freshData;
          updateMainUI(freshData, freshData.sessions.find(function(s) { return !s.shutdownTime; }) || null);
          refreshAdmin();
          refreshMainTable();
          refreshChart();
          renderHeatmap();
          msg.textContent = '还原完成';
        });
      } else {
        msg.textContent = '还原失败: ' + (res.error || '未知错误');
        msg.className = 'form-msg error';
      }
    })
    .catch(function() {
      msg.textContent = '网络错误，请重试';
      msg.className = 'form-msg error';
    });
}

function deleteBackup(encodedFilename, btnEl) {
  var filename = decodeURIComponent(encodedFilename);
  if (!confirm('确定删除备份文件？\n\n' + filename + '\n\n此操作不可恢复。')) return;

  if (btnEl) { btnEl.disabled = true; btnEl.textContent = '删除中...'; }

  fetch(_serverUrl('/api/delete-backup?filename=' + encodedFilename), { method: 'DELETE' })
    .then(function(r) { return r.json(); })
    .then(function(res) {
      if (res.ok) {
        // 删除成功后移除该行
        var row = btnEl ? btnEl.closest('tr') : null;
        if (row) row.remove();
        // 检查是否还有行
        var tbody = document.querySelector('#backupListWrap tbody');
        if (tbody && tbody.children.length === 0) {
          document.getElementById('backupListWrap').innerHTML = '<div class="admin-empty">暂无备份文件</div>';
        }
      } else {
        alert('删除失败: ' + (res.error || '未知错误'));
        if (btnEl) { btnEl.disabled = false; btnEl.textContent = '删除'; }
      }
    })
    .catch(function(err) {
      alert('网络错误: ' + err.message);
      if (btnEl) { btnEl.disabled = false; btnEl.textContent = '删除'; }
    });
}

function cleanOldBackups() {
  if (!confirm('确定清理旧备份？\n\n将只保留最近 10 个备份文件，其余全部删除。\n此操作不可恢复。')) return;

  var msg = document.getElementById('backupFormMsg');
  msg.textContent = '正在清理...';
  msg.className = 'form-msg';

  fetch(_serverUrl('/api/clean-backups'), { method: 'POST' })
    .then(function(r) { return r.json(); })
    .then(function(res) {
      if (res.ok) {
        msg.textContent = '已清理 ' + res.deleted + ' 个备份，保留 ' + res.remaining + ' 个';
        msg.className = 'form-msg success';
        loadBackupList();
      } else {
        msg.textContent = '清理失败: ' + (res.error || '未知错误');
        msg.className = 'form-msg error';
      }
    })
    .catch(function(err) {
      msg.textContent = '网络错误: ' + err.message;
      msg.className = 'form-msg error';
    });
}

// ——— 添加记录 ———
function addRecord() {
  var bootInput = document.getElementById('addBootTime');
  var shutInput = document.getElementById('addShutdownTime');
  var msg = document.getElementById('addFormMsg');

  if (!bootInput.value) {
    msg.textContent = '请填写开机时间';
    msg.className = 'form-msg error';
    return;
  }

  var bootTime = new Date(bootInput.value).toISOString();
  var shutdownTime = shutInput.value ? new Date(shutInput.value).toISOString() : null;

  // 校验：关机时间不能早于开机时间
  if (shutdownTime && shutdownTime <= bootTime) {
    msg.textContent = '关机时间不能早于或等于开机时间';
    msg.className = 'form-msg error';
    return;
  }

  if (_isServerAvailable() && window._serverReady) {
    fetch(_serverUrl('/api/add-session'), {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ bootTime: bootTime, shutdownTime: shutdownTime })
    }).then(function(r) { return r.json(); })
      .then(function(res) {
        if (res.ok) {
          // 从服务端响应拿回最新数据，直接更新缓存和 UI，不刷新
          var freshData = res.data || loadData();
          window._bootDataCache = freshData;
          try { localStorage.setItem(STORAGE_KEY, JSON.stringify(freshData)); } catch(e) {}
          msg.textContent = '记录已添加';
          msg.className = 'form-msg success';
          bootInput.value = '';
          shutInput.value = '';
          updateMainUI(freshData, freshData.sessions.find(function(s) { return !s.shutdownTime; }) || null);
          refreshAdmin();
          refreshMainTable();
          refreshChart();
          renderHeatmap();
          setTimeout(function() { msg.textContent = ''; }, 2000);
        } else {
          msg.textContent = res.error || '添加失败';
          msg.className = 'form-msg error';
        }
      }).catch(function() { msg.textContent = '网络错误，请重试'; msg.className = 'form-msg error'; });
  } else {
    // 纯浏览器模式
    var data = loadData();
    var session = {
      id: genId(),
      bootTime: bootTime,
      shutdownTime: shutdownTime,
      duration: null
    };
    if (shutdownTime) {
      session.duration = new Date(shutdownTime) - new Date(bootTime);
    }
    data.sessions.push(session);
    data.sessions.sort(function(a, b) { return (a.bootTime || '').localeCompare(b.bootTime || ''); });
    data.bootCount = data.sessions.length;
    data.shutdownCount = data.sessions.filter(function(s) { return s.shutdownTime; }).length;
    saveData(data);

    msg.textContent = '记录已添加';
    msg.className = 'form-msg success';
    bootInput.value = '';
    shutInput.value = '';

    updateMainUI(data, data.sessions.find(function(s) { return !s.shutdownTime; }) || null);
    refreshAdmin();
    refreshMainTable();
    refreshChart();
    renderHeatmap();
    setTimeout(function() { msg.textContent = ''; }, 2000);
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
  a.download = '开机记录_' + getLocalToday() + '.csv';
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
  a.download = '开机记录_' + getLocalToday() + '.json';
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
  var fileName = '开机记录_' + getLocalToday() + '.xlsx';
  XLSX.writeFile(wb, fileName);
}

// ——— 暗色/浅色切换 ———
function toggleMode() {
  var isLight = document.body.classList.toggle('light-mode');
  var btn = document.getElementById('modeToggle');
  var icon = btn.querySelector('use');
  icon.setAttribute('href', 'static/icons/icons.svg#icon-' + (isLight ? 'moon' : 'sun'));
  btn.title = isLight ? '切换暗色模式' : '切换浅色模式';
  localStorage.setItem('bootTrackerMode', isLight ? 'light' : 'dark');
  // 刷新图表和热度图（内联样式颜色需重绘）
  refreshChart();
  renderHeatmap();
}

// 加载保存的模式
(function() {
  var saved = localStorage.getItem('bootTrackerMode');
  if (saved === 'light') {
    document.body.classList.add('light-mode');
    var icon = document.getElementById('modeToggle').querySelector('use');
    icon.setAttribute('href', 'static/icons/icons.svg#icon-moon');
    document.getElementById('modeToggle').title = '切换暗色模式';
  }
})();

// ——— 主题切换 ———
function switchTheme(name) {
  document.documentElement.setAttribute('data-theme', name);
  localStorage.setItem('bootTrackerTheme', name);
  // 更新激活状态
  document.querySelectorAll('.theme-card').forEach(function(c) {
    c.classList.toggle('active', c.getAttribute('data-theme') === name);
  });
  // 更新模式按钮
  updateModeButtons();
  // 刷新图表和热度图（颜色随主题变化）
  refreshChart();
  renderHeatmap();
}

// 加载保存的主题
(function() {
  var saved = localStorage.getItem('bootTrackerTheme') || 'mica';
  switchTheme(saved);
})();

// ——— 主题面板 ———
function toggleSettingsPanel() {
  var overlay = document.getElementById('settingsOverlay');
  var isActive = overlay.classList.toggle('active');
  if (isActive) {
    loadVersionInfo();
    loadAppSettings();
  }
}

function pickTheme(name) {
  switchTheme(name);
}

function setMode(mode) {
  if (mode === 'light') {
    document.body.classList.add('light-mode');
    document.getElementById('modeToggle').textContent = '\u263E';
    document.getElementById('modeToggle').title = '切换暗色模式';
  } else {
    document.body.classList.remove('light-mode');
    document.getElementById('modeToggle').textContent = '\u2600';
    document.getElementById('modeToggle').title = '切换浅色模式';
  }
  localStorage.setItem('bootTrackerMode', mode);
  updateModeButtons();
  refreshChart();
}

function updateModeButtons() {
  var isLight = document.body.classList.contains('light-mode');
  var dark = document.getElementById('themeModeDark');
  var light = document.getElementById('themeModeLight');
  if (dark && light) {
    dark.classList.toggle('active', !isLight);
    light.classList.toggle('active', isLight);
  }
}

// 初始化模式按钮状态
(function() { updateModeButtons(); })();

// ——— 快捷键（备用） ———
document.addEventListener('keydown', function(e) {
  if (e.ctrlKey && e.shiftKey && e.key === 'A') {
    e.preventDefault();
    toggleAdmin();
  }
  if (e.key === 'Escape') closeEditModal();
});

// ——— 点击三次开机次数进入管理员 ———
var adminClickCount = 0;
var adminClickTimer = null;
document.getElementById('footerVersion').addEventListener('click', function() {
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
  try {
    var data = loadData();
    var fromEl = document.getElementById('filterFrom');
    var toEl = document.getElementById('filterTo');
    var from = fromEl ? fromEl.value : '';
    var to = toEl ? toEl.value : '';
    return data.sessions.filter(function(s) {
      var d = isoToLocalDate(s.bootTime);
      if (from && d < from) return false;
      if (to && d > to) return false;
      return true;
    });
  } catch(e) {
    return (loadData().sessions || []).slice();
  }
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
var chartTooltip = null;

function switchMainTab(tab) {
  document.querySelectorAll('.main-tab').forEach(function(b) {
    b.classList.toggle('active', b.id === 'tab' + tab.charAt(0).toUpperCase() + tab.slice(1));
  });
  document.querySelectorAll('.page').forEach(function(p) {
    p.classList.toggle('active', p.id === 'page' + tab.charAt(0).toUpperCase() + tab.slice(1));
  });
  if (tab === 'records') {
    refreshMainTable();
  } else if (tab === 'chart') {
    refreshChart();
    renderHeatmap();
  }
}

function switchChart(type) {
  currentChartType = type;
  document.querySelectorAll('.chart-tab').forEach(function(b) {
    b.classList.toggle('active', (type === 'bar' && b.textContent === '柱状图') || 
                                  (type === 'line' && b.textContent === '折线图') ||
                                  (type === 'pie' && b.textContent === '饼图'));
  });
  refreshChart();
}

function _showChartTooltip(svg, x, y, date, value, unit) {
  if (!chartTooltip) {
    chartTooltip = document.createElement('div');
    chartTooltip.className = 'chart-tooltip';
    document.body.appendChild(chartTooltip);
  }
  
  var svgRect = svg.getBoundingClientRect();
  var tooltipX = svgRect.left + x + 10;
  var tooltipY = svgRect.top + y - 40;
  
  if (tooltipX + 150 > window.innerWidth) {
    tooltipX = svgRect.left + x - 160;
  }
  if (tooltipY < 10) {
    tooltipY = svgRect.top + y + 20;
  }
  
  chartTooltip.style.left = tooltipX + 'px';
  chartTooltip.style.top = tooltipY + 'px';
  chartTooltip.innerHTML = '<div class="tooltip-date">' + date + '</div><div class="tooltip-value">' + value + ' ' + unit + '</div>';
  chartTooltip.style.display = 'block';
  setTimeout(function() {
    chartTooltip.classList.add('visible');
  }, 10);
}

function _hideChartTooltip() {
  if (chartTooltip) {
    chartTooltip.classList.remove('visible');
    setTimeout(function() {
      chartTooltip.style.display = 'none';
    }, 150);
  }
}

var _statsCache = null;

function loadStatsData(callback) {
  if (!_isServerAvailable() || !window._serverReady) {
    callback(null);
    return;
  }
  
  if (_statsCache && Date.now() - _statsCache.timestamp < 60000) {
    callback(_statsCache.data);
    return;
  }
  
  fetch(_serverUrl('/api/stats/daily'))
    .then(function(r) { return r.json(); })
    .then(function(res) {
      if (res && res.data) {
        _statsCache = { data: res.data, timestamp: Date.now() };
        callback(res.data);
      } else {
        callback(null);
      }
    })
    .catch(function() {
      callback(null);
    });
}

function refreshChart() {
  var svg = document.getElementById('chartSVG');
  if (!svg) return;
  
  var style = getComputedStyle(document.documentElement);
  var accent = style.getPropertyValue('--accent').trim() || '#8b5cf6';
  var accentRgb = style.getPropertyValue('--accent-rgb').trim() || '139,92,246';
  var axisColor = document.body.classList.contains('light-mode') ? '#64748b' : '#94a3b8';
  var bgColor = document.body.classList.contains('light-mode') ? '#ffffff' : '#0a0c14';
  var textColor = document.body.classList.contains('light-mode') ? '#1e293b' : '#f1f5f9';
  
  svg.innerHTML = '<text x="400" y="150" text-anchor="middle" fill="' + axisColor + '" font-family="Inter,sans-serif" font-size="14">加载中...</text>';
  
  var sessions = getFilteredSessions();
  
  if (sessions.length === 0) {
    svg.innerHTML = '<text x="400" y="150" text-anchor="middle" fill="' + axisColor + '" font-family="Inter,sans-serif" font-size="14">暂无数据</text>';
    return;
  }

  var W = 800, H = 340;
  var padL = 55, padR = 20, padT = 55, padB = 50;
  var plotW = W - padL - padR;
  var plotH = H - padT - padB;

  if (currentChartType === 'pie') {
    _renderPieChart(svg, sessions, W, H, padL, padR, padT, padB, plotW, plotH, accent, accentRgb, axisColor, textColor);
    return;
  }
  
  loadStatsData(function(statsData) {
    _renderLineBarChart(svg, sessions, statsData, W, H, padL, padR, padT, padB, plotW, plotH, accent, accentRgb, axisColor, textColor);
  });
}

function _renderLineBarChart(svg, sessions, statsData, W, H, padL, padR, padT, padB, plotW, plotH, accent, accentRgb, axisColor, textColor) {

  var now = new Date();
  var byDate = {};
  sessions.forEach(function(s) {
    var day = isoToLocalDate(s.bootTime);
    if (!byDate[day]) byDate[day] = { duration: 0, count: 0 };
    var dur = s.duration || (now - new Date(s.bootTime));
    byDate[day].duration += dur;
    byDate[day].count += 1;
  });
  var dates = Object.keys(byDate).sort();
  var values = dates.map(function(d) { return Math.round(byDate[d].duration / 60000); });

  var maxVal = Math.max.apply(null, values) || 1;
  maxVal = Math.ceil(maxVal * 1.15);

  var cols = dates.length;
  var barW = Math.min(40, Math.max(6, (plotW / cols) * 0.7));
  var gap = plotW / cols;

  var svgHTML = '';

  svgHTML += '<rect x="0" y="0" width="' + W + '" height="' + H + '" fill="none"/>';

  svgHTML += '<text x="' + (W / 2) + '" y="22" text-anchor="middle" fill="' + textColor + '" font-family="Inter,sans-serif" font-size="16" font-weight="600">每日开机时长统计</text>';

  var legendY = 38;
  svgHTML += '<circle cx="' + (W / 2 - 40) + '" cy="' + legendY + '" r="4" fill="rgba(' + accentRgb + ',0.5)"/>';
  svgHTML += '<text x="' + (W / 2 - 32) + '" y="' + (legendY + 4) + '" fill="' + axisColor + '" font-family="Inter,sans-serif" font-size="11">开机时长（分钟）</text>';

  var gridLines = 5;
  for (var i = 0; i <= gridLines; i++) {
    var y = padT + (plotH / gridLines) * i;
    var val = Math.round(maxVal - (maxVal / gridLines) * i);
    svgHTML += '<line x1="' + padL + '" y1="' + y + '" x2="' + (padL + plotW) + '" y2="' + y + '" stroke="rgba(' + accentRgb + ',0.06)" stroke-width="1"/>';
    svgHTML += '<text x="' + (padL - 8) + '" y="' + (y + 4) + '" text-anchor="end" fill="' + axisColor + '" font-family="Inter,sans-serif" font-size="10">' + val + '</text>';
  }

  var points = [];
  var pathD = '';
  
  var totalCount = sessions.length;
  var completedCount = sessions.filter(function(s) { return s.shutdownTime; }).length;
  var avgDuration = completedCount > 0 ? Math.round(sessions.reduce(function(a, s) { return a + (s.duration || 0); }, 0) / completedCount / 60000) : 0;
  var maxDuration = values.length > 0 ? Math.max.apply(null, values) : 0;
  var minDuration = values.length > 0 ? Math.min.apply(null, values) : 0;
  
  var statY = padT + plotH + 35;
  svgHTML += '<text x="' + padL + '" y="' + statY + '" fill="' + axisColor + '" font-family="Inter,sans-serif" font-size="10">';
  svgHTML += '<tspan fill="' + accent + '">总记录: ' + totalCount + '</tspan>  ';
  svgHTML += '<tspan fill="' + accent + '">已完成: ' + completedCount + '</tspan>  ';
  svgHTML += '<tspan fill="' + accent + '">平均时长: ' + avgDuration + '分钟</tspan>';
  svgHTML += '</text>';
  
  var statY2 = padT + plotH + 48;
  svgHTML += '<text x="' + padL + '" y="' + statY2 + '" fill="' + axisColor + '" font-family="Inter,sans-serif" font-size="9">';
  svgHTML += '<tspan>最高: ' + maxDuration + '分钟</tspan>  ';
  svgHTML += '<tspan>最低: ' + minDuration + '分钟</tspan>';
  svgHTML += '</text>';

  values.forEach(function(v, idx) {
    var cx = padL + gap * idx + gap / 2;
    var cy = padT + plotH - (v / maxVal) * plotH;
    points.push({ x: cx, y: cy, v: v, date: dates[idx], count: byDate[dates[idx]].count });

    if (currentChartType === 'bar') {
      var barHeight = padT + plotH - cy;
      var gradientId = 'barGradient' + idx;
      svgHTML += '<defs><linearGradient id="' + gradientId + '" x1="0%" y1="0%" x2="0%" y2="100%">';
      svgHTML += '<stop offset="0%" stop-color="rgba(' + accentRgb + ',0.65)"/>';
      svgHTML += '<stop offset="100%" stop-color="rgba(' + accentRgb + ',0.35)"/>';
      svgHTML += '</linearGradient></defs>';
      svgHTML += '<rect x="' + (cx - barW/2) + '" y="' + cy + '" width="' + barW + '" height="' + barHeight + '" rx="4" fill="url(#' + gradientId + ')"';
      svgHTML += ' onmouseenter="_showChartTooltip(this.closest(\'svg\'), ' + cx + ', ' + cy + ', \'' + dates[idx] + '\', ' + v + ', \'分钟\')"';
      svgHTML += ' onmouseleave="_hideChartTooltip()" style="cursor:pointer;transition:all 0.2s;"/>';
    }

    if (idx === 0) pathD = 'M' + cx + ',' + cy;
    else pathD += ' L' + cx + ',' + cy;
  });

  if (currentChartType === 'line') {
    var smoothPath = _smoothPath(points, plotH, padT);
    var areaD = smoothPath + ' L' + (padL + plotW) + ',' + (padT + plotH) + ' L' + padL + ',' + (padT + plotH) + ' Z';
    svgHTML += '<defs><linearGradient id="lineAreaGradient" x1="0%" y1="0%" x2="0%" y2="100%">';
    svgHTML += '<stop offset="0%" stop-color="rgba(' + accentRgb + ',0.25)"/>';
    svgHTML += '<stop offset="100%" stop-color="rgba(' + accentRgb + ',0.02)"/>';
    svgHTML += '</linearGradient></defs>';
    svgHTML += '<path d="' + areaD + '" fill="url(#lineAreaGradient)" style="animation: fadeIn 0.5s ease;"/>';
    svgHTML += '<path d="' + smoothPath + '" fill="none" stroke="' + accent + '" stroke-width="3" stroke-linecap="round" stroke-linejoin="round" style="animation: drawLine 0.8s ease forwards;stroke-dasharray:2000;stroke-dashoffset:2000;"/>';
  }

  points.forEach(function(p) {
    var r = currentChartType === 'line' ? 5 : 0;
    var hoverR = r + 3;
    svgHTML += '<circle cx="' + p.x + '" cy="' + p.y + '" r="' + hoverR + '" fill="rgba(' + accentRgb + ',0.15)"';
    svgHTML += ' onmouseenter="_showChartTooltip(this.closest(\'svg\'), ' + p.x + ', ' + p.y + ', \'' + p.date + '\', ' + p.v + ', \'分钟\')"';
    svgHTML += ' onmouseleave="_hideChartTooltip()" style="cursor:pointer;"/>';
    svgHTML += '<circle cx="' + p.x + '" cy="' + p.y + '" r="' + r + '" fill="' + accent + '"';
    svgHTML += ' onmouseenter="_showChartTooltip(this.closest(\'svg\'), ' + p.x + ', ' + p.y + ', \'' + p.date + '\', ' + p.v + ', \'分钟\')"';
    svgHTML += ' onmouseleave="_hideChartTooltip()" style="cursor:pointer;transition:r 0.2s;"/>';
  });

  var step = Math.max(1, Math.floor(cols / 8));
  dates.forEach(function(d, idx) {
    if (idx % step === 0 || idx === dates.length - 1) {
      var cx = padL + gap * idx + gap / 2;
      var label = d.slice(5);
      svgHTML += '<text x="' + cx + '" y="' + (H - 10) + '" text-anchor="middle" fill="' + axisColor + '" font-family="Inter,sans-serif" font-size="9">' + label + '</text>';
    }
  });

  svgHTML += '<text x="8" y="' + (padT + plotH/2) + '" text-anchor="middle" fill="' + axisColor + '" font-family="Inter,sans-serif" font-size="10" transform="rotate(-90,8,' + (padT + plotH/2) + ')">时长（分钟）</text>';

  svg.innerHTML = svgHTML;
  
  if (currentChartType === 'line') {
    var styleSheet = document.createElement('style');
    styleSheet.textContent = '@keyframes drawLine {to{stroke-dashoffset:0;}}';
    document.head.appendChild(styleSheet);
    setTimeout(function() { document.head.removeChild(styleSheet); }, 1000);
  }
}

function _smoothPath(points, height, offset) {
  if (points.length < 2) return '';
  var path = 'M' + points[0].x + ',' + points[0].y;
  for (var i = 0; i < points.length - 1; i++) {
    var p0 = points[i - 1] || points[i];
    var p1 = points[i];
    var p2 = points[i + 1];
    var p3 = points[i + 2] || p2;
    
    var cp1x = p1.x + (p2.x - p0.x) / 6;
    var cp1y = p1.y + (p2.y - p0.y) / 6;
    var cp2x = p2.x - (p3.x - p1.x) / 6;
    var cp2y = p2.y - (p3.y - p1.y) / 6;
    
    path += ' C' + cp1x + ',' + cp1y + ' ' + cp2x + ',' + cp2y + ' ' + p2.x + ',' + p2.y;
  }
  return path;
}

function _renderPieChart(svg, sessions, W, H, padL, padR, padT, padB, plotW, plotH, accent, accentRgb, axisColor, textColor) {
  var now = new Date();
  var byDate = {};
  sessions.forEach(function(s) {
    var day = isoToLocalDate(s.bootTime);
    if (!byDate[day]) byDate[day] = 0;
    byDate[day] += 1;
  });

  var dates = Object.keys(byDate).sort();
  var topDates = dates.slice(-7);
  var topValues = topDates.map(function(d) { return byDate[d]; });
  var otherCount = dates.length > 7 ? dates.slice(0, -7).reduce(function(a, d) { return a + byDate[d]; }, 0) : 0;

  if (topDates.length === 0) {
    svg.innerHTML = '<text x="400" y="150" text-anchor="middle" fill="' + axisColor + '" font-family="Inter,sans-serif" font-size="14">暂无数据</text>';
    return;
  }

  if (otherCount > 0) {
    topDates.push('其他');
    topValues.push(otherCount);
  }

  var centerX = W / 2;
  var centerY = (H - padT - padB) / 2 + padT;
  var radius = Math.min(plotW, plotH) / 2 - 30;
  var innerRadius = radius * 0.55;

  var colors = [
    'rgba(' + accentRgb + ',0.9)',
    'rgba(59,130,246,0.9)',
    'rgba(16,185,129,0.9)',
    'rgba(245,158,11,0.9)',
    'rgba(239,68,68,0.9)',
    'rgba(139,92,246,0.65)',
    'rgba(59,130,246,0.65)',
    'rgba(100,116,139,0.65)',
  ];

  var svgHTML = '';
  svgHTML += '<rect x="0" y="0" width="' + W + '" height="' + H + '" fill="none"/>';
  svgHTML += '<text x="' + centerX + '" y="22" text-anchor="middle" fill="' + textColor + '" font-family="Inter,sans-serif" font-size="16" font-weight="600">开机次数分布（最近7天）</text>';

  var total = topValues.reduce(function(a, b) { return a + b; }, 0);
  var startAngle = -Math.PI / 2;

  var legendX = centerX + radius + 35;
  var legendY = centerY - 50;

  topValues.forEach(function(val, idx) {
    var angle = (val / total) * 2 * Math.PI;
    var endAngle = startAngle + angle;

    var x1 = centerX + radius * Math.cos(startAngle);
    var y1 = centerY + radius * Math.sin(startAngle);
    var x2 = centerX + radius * Math.cos(endAngle);
    var y2 = centerY + radius * Math.sin(endAngle);

    var innerX1 = centerX + innerRadius * Math.cos(startAngle);
    var innerY1 = centerY + innerRadius * Math.sin(startAngle);
    var innerX2 = centerX + innerRadius * Math.cos(endAngle);
    var innerY2 = centerY + innerRadius * Math.sin(endAngle);

    var largeArc = angle > Math.PI ? 1 : 0;
    var path = 'M' + centerX + ',' + centerY + ' L' + x1 + ',' + y1 + ' A' + radius + ',' + radius + ' 0 ' + largeArc + ',1 ' + x2 + ',' + y2 + ' L' + centerX + ',' + centerY + ' Z';
    var donutPath = 'M' + innerX1 + ',' + innerY1 + ' L' + x1 + ',' + y1 + ' A' + radius + ',' + radius + ' 0 ' + largeArc + ',1 ' + x2 + ',' + y2 + ' L' + innerX2 + ',' + innerY2 + ' A' + innerRadius + ',' + innerRadius + ' 0 ' + largeArc + ',0 ' + innerX1 + ',' + innerY1 + ' Z';

    svgHTML += '<path d="' + donutPath + '" fill="' + colors[idx] + '" stroke="' + (document.body.classList.contains('light-mode') ? '#f1f5f9' : '#1e293b') + '" stroke-width="2"';
    svgHTML += ' onmouseenter="_showChartTooltip(this.closest(\'svg\'), ' + centerX + ', ' + centerY + ', \'' + topDates[idx] + '\', ' + val + ', \'次\')"';
    svgHTML += ' onmouseleave="_hideChartTooltip()" style="cursor:pointer;transition:all 0.2s;"/>';

    svgHTML += '<rect x="' + legendX + '" y="' + (legendY + idx * 24) + '" width="12" height="12" rx="2" fill="' + colors[idx] + '"';
    svgHTML += ' onmouseenter="_showChartTooltip(this.closest(\'svg\'), ' + centerX + ', ' + centerY + ', \'' + topDates[idx] + '\', ' + val + ', \'次\')"';
    svgHTML += ' onmouseleave="_hideChartTooltip()" style="cursor:pointer;"/>';
    svgHTML += '<text x="' + (legendX + 18) + '" y="' + (legendY + idx * 24 + 9) + '" fill="' + axisColor + '" font-family="Inter,sans-serif" font-size="11">';
    var percent = Math.round((val / total) * 100);
    svgHTML += topDates[idx] + ' (' + val + '次, ' + percent + '%)';
    svgHTML += '</text>';

    startAngle = endAngle;
  });

  svgHTML += '<circle cx="' + centerX + '" cy="' + centerY + '" r="' + innerRadius + '" fill="' + (document.body.classList.contains('light-mode') ? '#ffffff' : '#0a0c14') + '" stroke="none"/>';
  
  svgHTML += '<text x="' + centerX + '" y="' + (centerY - 8) + '" text-anchor="middle" fill="' + textColor + '" font-family="Inter,sans-serif" font-size="24" font-weight="700">' + total + '</text>';
  svgHTML += '<text x="' + centerX + '" y="' + (centerY + 14) + '" text-anchor="middle" fill="' + axisColor + '" font-family="Inter,sans-serif" font-size="11">总开机次数</text>';
  
  var maxDay = topValues.indexOf(Math.max.apply(null, topValues));
  var statY = centerY + radius + 25;
  svgHTML += '<text x="' + centerX + '" y="' + statY + '" fill="' + axisColor + '" font-family="Inter,sans-serif" font-size="10" text-anchor="middle">';
  svgHTML += '<tspan fill="' + accent + '">最多: ' + topDates[maxDay] + ' (' + topValues[maxDay] + '次)</tspan>';
  svgHTML += '</text>';

  svg.innerHTML = svgHTML;
}

function refreshMainTable() {
  var sessions = getFilteredSessions();
  var tbody = document.getElementById('mainRecordsBody');
  var empty = document.getElementById('mainEmptyMsg');

  if (sessions.length === 0) {
    tbody.innerHTML = '';
    // 如果没有记录但缓存有数据，说明加载失败，显示错误提示
    var cached = window._bootDataCache;
    if (cached && cached.sessions && cached.sessions.length > 0) {
      empty.innerHTML = '数据加载中，请稍候...<br><small style="opacity:0.5">若持续显示，请重启应用</small>';
    } else {
      empty.innerHTML = '暂无开机记录';
    }
    empty.style.display = 'block';
    return;
  }
  empty.style.display = 'none';

  var sorted = [].concat(sessions).reverse();
  var total = (loadData().sessions || []).length;
  var dayInfo = _groupByDate(sorted);
  var allData = loadData();
  var globalIdx = total - sessions.length;

  var html = '';
  dayInfo.order.reverse().forEach(function(dateStr) {
    var group = dayInfo.groups[dateStr];
    var collapsed = _collapsedDays.hasOwnProperty(dateStr) ? !!_collapsedDays[dateStr] : true;
    var cls = 'day-header-' + dateStr.replace(/-/g, '_');
    var dur = _dayTotalDuration(group);
    var hasActive = group.some(function(s) { return !s.shutdownTime; });

    html += '<tr class="day-header ' + cls + (collapsed ? ' collapsed' : '') + '" onclick="toggleDayGroup(\'' + dateStr + '\', \'main\')">' +
      '<td colspan="5">' +
        '<span class="day-arrow">' + (collapsed ? '&#9654;' : '&#9660;') + '</span>' +
        '<span class="day-label">' + _dayLabel(dateStr) + '</span>' +
        '<span class="day-count">' + group.length + '条记录</span>' +
        '<span class="day-duration">' + fmtDuration(dur) + '</span>' +
        (hasActive ? '<span class="status-dot active day-active-dot"></span>' : '') +
      '</td>' +
    '</tr>';

    group.forEach(function(s) {
      globalIdx++;
      var isActive = !s.shutdownTime;
      var gcls = 'day-group-' + dateStr.replace(/-/g, '_');
      html += '<tr class="session-row ' + gcls + '"' + (collapsed ? ' style="display:none"' : '') + '>' +
        '<td>' + globalIdx + '</td>' +
        '<td>' + fmtTime(s.bootTime) + '</td>' +
        '<td>' + (isActive ? '\u2014' : fmtTime(s.shutdownTime)) + '</td>' +
        '<td>' + (isActive ? '\u2014' : fmtDuration(s.duration)) + '</td>' +
        '<td>' + (isActive
          ? '<span class="status-dot active"></span>\u8fdb\u884c\u4e2d'
          : '<span class="status-dot closed"></span>\u5df2\u5173\u673a') + '</td>' +
      '</tr>';
    });
  });

  tbody.innerHTML = html;
  renderHeatmap();
}


// ——— 热度图渲染 ———
function getHeatColor(count, max) {
  var isMica = document.documentElement.getAttribute('data-theme') === 'mica';
  var isLight = document.body.classList.contains('light-mode');
  var style = getComputedStyle(document.documentElement);
  var accentRgb = style.getPropertyValue('--accent-rgb').trim() || '139,92,246';
  if (count === 0) {
    if (isMica) return 'rgba(255,255,255,0.03)';
    return isLight ? 'rgba(0,0,0,0.04)' : 'rgba(30,41,59,0.5)';
  }
  var r = count / max;
  if (isMica) {
    // 云母：灰阶热力
    if (r <= 0.25) return 'rgba(148,163,184,0.25)';
    if (r <= 0.5) return 'rgba(148,163,184,0.4)';
    if (r <= 0.75) return 'rgba(148,163,184,0.6)';
    return 'rgba(148,163,184,0.8)';
  }
  if (isLight) {
    // 亮色模式：使用主题色
    if (r <= 0.25) return 'rgba(' + accentRgb + ',0.12)';
    if (r <= 0.5) return 'rgba(' + accentRgb + ',0.25)';
    if (r <= 0.75) return 'rgba(' + accentRgb + ',0.45)';
    return 'rgba(' + accentRgb + ',0.7)';
  }
  // 暗色模式：使用主题色
  if (r <= 0.25) return 'rgba(' + accentRgb + ',0.5)';
  if (r <= 0.5) return 'rgba(' + accentRgb + ',0.65)';
  if (r <= 0.75) return 'rgba(' + accentRgb + ',0.8)';
  return 'rgba(' + accentRgb + ',0.95)';
}

function renderHeatmap() {
  var container = document.getElementById('heatmapContainer');
  var sessions = getFilteredSessions();
  // 动态文本颜色（随亮/暗模式变化）
  var isLight = document.body.classList.contains('light-mode');
  var txtColor = isLight ? '#64748b' : '#94a3b8';

  // 更新图例颜色（跟随主题）
  var style = getComputedStyle(document.documentElement);
  var accentRgb = style.getPropertyValue('--accent-rgb').trim() || '139,92,246';
  var legendOpacitys = ['0.5', '0.65', '0.8', '0.95'];
  for (var li = 0; li < 4; li++) {
    var el = document.getElementById('hmLegend' + (li + 1));
    if (el) el.style.background = 'rgba(' + accentRgb + ',' + legendOpacitys[li] + ')';
  }

  if (!sessions || sessions.length === 0) {
    container.innerHTML = '<div style="text-align:center;color:' + txtColor + ';padding:40px;font-size:0.85rem;">暂无数据</div>';
    return;
  }

  // 统计每天开机次数
  var byDay = {};
  sessions.forEach(function(s) {
    var day = isoToLocalDate(s.bootTime);
    byDay[day] = (byDay[day] || 0) + 1;
  });

  var dates = Object.keys(byDay).sort();
  if (dates.length === 0) {
    container.innerHTML = '<div style="text-align:center;color:' + txtColor + ';padding:40px;font-size:0.85rem;">暂无数据</div>';
    return;
  }

  var maxCount = 0;
  for (var d in byDay) { if (byDay[d] > maxCount) maxCount = byDay[d]; }
  if (maxCount === 0) maxCount = 1;

  var months = ['1月','2月','3月','4月','5月','6月','7月','8月','9月','10月','11月','12月'];
  var dayNames = ['日','一','二','三','四','五','六'];
  var CELL = 14, GAP = 3;

  // 日期范围：至少显示最近 12 周
  var p0 = dates[0].split('-');
  var p1 = dates[dates.length-1].split('-');
  var firstDate = new Date(parseInt(p0[0]), parseInt(p0[1]) - 1, parseInt(p0[2]));
  var lastDate = new Date(parseInt(p1[0]), parseInt(p1[1]) - 1, parseInt(p1[2]));
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
    var key = isoToLocalDate(c.toISOString());
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
      monthRow += '<div style="width:' + (CELL + GAP) + 'px;flex-shrink:0;font-size:0.65rem;color:' + txtColor + ';">' + months[monthSections[i]] + '</div>';
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
          '<div style="display:flex;flex-direction:column;gap:' + GAP + 'px;font-size:0.65rem;color:' + txtColor + ';width:26px;flex-shrink:0;padding-top:0;">' +
            dayNames.map(function(d) { return '<div style="height:' + CELL + 'px;line-height:' + CELL + 'px;text-align:right;padding-right:4px;">' + d + '</div>'; }).join('') +
          '</div>' +
          '<div style="display:flex;gap:' + GAP + 'px;">' + weekCols + '</div>' +
        '</div>' +
      '</div>' +
    '</div>';
}

// ——— 编辑记录 ———
var _editingSessionId = null;

function openEditModal(id) {
  var data = loadData();
  var session = data.sessions.find(function(s) { return s.id === id; });
  if (!session) return;
  _editingSessionId = id;
  document.getElementById('editBootTime').value = isoToLocalInput(session.bootTime);
  document.getElementById('editShutdownTime').value = session.shutdownTime ? isoToLocalInput(session.shutdownTime) : '';
  document.getElementById('editModal').classList.add('active');
}

function closeEditModal() {
  document.getElementById('editModal').classList.remove('active');
  _editingSessionId = null;
}

function saveEdit() {
  if (!_editingSessionId) return;
  var bootVal = document.getElementById('editBootTime').value;
  var shutVal = document.getElementById('editShutdownTime').value;
  if (!bootVal) { alert('开机时间不能为空'); return; }
  var bootTime = localInputToIso(bootVal);
  var shutdownTime = shutVal ? localInputToIso(shutVal) : null;

  if (_isServerAvailable() && window._serverReady) {
    fetch(_serverUrl('/api/sessions/' + _editingSessionId), {
      method: 'PUT',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ bootTime: bootTime, shutdownTime: shutdownTime })
    }).then(function(r) { return r.json(); })
      .then(function(res) {
        if (res.ok) {
          loadDataAsync(function(freshData) {
            closeEditModal();
            window._bootDataCache = freshData;
            var activeSession = freshData.sessions.find(function(s) { return !s.shutdownTime; }) || null;
            updateMainUI(freshData, activeSession);
            // 同步计时器：如果编辑的是当前活跃会话，重启计时器；如果被标记为关机，停止计时器
            if (activeSession && activeSession.id === _editingSessionId) {
              startTimer(activeSession.bootTime);
            } else if (!activeSession || activeSession.id !== _editingSessionId) {
              // 编辑的会话不再是活跃的（可能加了关机时间），检查是否需要停止计时器
              var wasActiveBefore = freshData.sessions.find(function(s) { return s.id === _editingSessionId && s.shutdownTime; });
              if (wasActiveBefore && timerInterval) { clearInterval(timerInterval); timerInterval = null; }
              // 如果还有其他活跃会话，用它的开机时间重启
              if (activeSession) startTimer(activeSession.bootTime);
            }
            refreshAdmin();
            refreshMainTable();
            refreshChart();
            renderHeatmap();
          });
        } else {
          alert('保存失败: ' + (res.error || '未知错误'));
        }
      }).catch(function() { alert('网络错误'); });
  } else {
    // 纯浏览器模式
    var data = loadData();
    var session = data.sessions.find(function(s) { return s.id === _editingSessionId; });
    if (!session) return;
    session.bootTime = bootTime;
    session.shutdownTime = shutdownTime;
    session.duration = shutdownTime ? (new Date(shutdownTime) - new Date(bootTime)) : null;
    data.bootCount = data.sessions.length;
    data.shutdownCount = data.sessions.filter(function(s) { return s.shutdownTime; }).length;
    window._bootDataCache = data;
    saveData(data);
    closeEditModal();
    var activeSession = data.sessions.find(function(s) { return !s.shutdownTime; }) || null;
    updateMainUI(data, activeSession);
    // 同步计时器
    if (activeSession && activeSession.id === _editingSessionId) {
      startTimer(activeSession.bootTime);
    } else {
      if (timerInterval) { clearInterval(timerInterval); timerInterval = null; }
      if (activeSession) startTimer(activeSession.bootTime);
    }
    refreshAdmin();
    refreshMainTable();
    refreshChart();
    renderHeatmap();
  }
}

function isoToLocalInput(iso) {
  if (!iso) return '';
  var d = new Date(iso);
  var pad = function(n) { return n < 10 ? '0' + n : '' + n; };
  return d.getFullYear() + '-' + pad(d.getMonth()+1) + '-' + pad(d.getDate()) + 'T'
    + pad(d.getHours()) + ':' + pad(d.getMinutes()) + ':' + pad(d.getSeconds());
}

function localInputToIso(val) {
  if (!val) return null;
  return new Date(val).toISOString();
}

// 编辑模态框遮罩点击关闭
document.getElementById('editModal').addEventListener('click', function(e) {
  if (e.target === this) closeEditModal();
});

// ——— 管理员表格事件委托（编辑+删除+复选框） ———
document.getElementById('adminRecordsBody').addEventListener('click', function(e) {
  var btn = e.target.closest('button');
  if (!btn) return;
  var id = btn.getAttribute('data-id');
  if (!id) return;
  if (btn.classList.contains('btn-edit')) openEditModal(id);
  else if (btn.classList.contains('btn-del')) adminDeleteRecord(id);
});

// 复选框 change 事件
document.getElementById('adminRecordsBody').addEventListener('change', function(e) {
  if (e.target.classList.contains('admin-check')) {
    e.target.closest('tr').classList.toggle('selected', e.target.checked);
    updateMergeBtn();
    var all = document.querySelectorAll('.admin-check');
    var checked = document.querySelectorAll('.admin-check:checked');
    var sa = document.getElementById('adminSelectAll');
    if (sa) sa.checked = all.length > 0 && all.length === checked.length;
  }
});

// 全选/取消
document.getElementById('adminSelectAll').addEventListener('change', function(e) {
  document.querySelectorAll('.admin-check').forEach(function(cb) {
    cb.checked = e.target.checked;
    cb.closest('tr').classList.toggle('selected', e.target.checked);
  });
  updateMergeBtn();
});

function updateMergeBtn() {
  var count = document.querySelectorAll('.admin-check:checked').length;
  var btn = document.getElementById('btnMerge');
  if (btn) {
    if (count >= 2) {
      btn.style.display = 'inline-block';
      btn.textContent = '\u{1F517} 合并选中 (' + count + ')';
    } else {
      btn.style.display = 'none';
    }
  }
}

// ——— 合并记录 ———
function mergeSelected() {
  var checked = document.querySelectorAll('.admin-check:checked');
  var ids = Array.from(checked).map(function(cb) { return cb.getAttribute('data-id'); });
  if (ids.length < 2) { alert('至少选择两条记录'); return; }
  if (!confirm('确定合并选中的 ' + ids.length + ' 条记录？\n合并后保留最早开机时间和最晚关机时间。\n原记录将移入回收站，可恢复。')) return;

  if (_isServerAvailable() && window._serverReady) {
    fetch(_serverUrl('/api/merge-sessions'), {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ ids: ids })
    }).then(function(r) { return r.json(); })
      .then(function(res) {
        if (res.ok) {
          var freshData = res.data || loadData();
          window._bootDataCache = freshData;
          try { localStorage.setItem(STORAGE_KEY, JSON.stringify(freshData)); } catch(e) {}
          var active = freshData.sessions.find(function(s) { return !s.shutdownTime; });
          if (active) sessionStorage.setItem('currentSessionId', active.id);
          else sessionStorage.removeItem('currentSessionId');
          updateMainUI(freshData, active || null);
          refreshAdmin();
          refreshMainTable();
          refreshChart();
          renderHeatmap();
        } else {
          alert('合并失败: ' + (res.error || '未知错误'));
        }
      }).catch(function() { alert('网络错误，请重试'); });
  } else {
    // 纯浏览器模式
    var data = loadData();
    var selected = data.sessions.filter(function(s) { return ids.indexOf(s.id) !== -1; });
    if (selected.length < 2) return;

    var bootTimes = selected.map(function(s) { return s.bootTime; }).sort();
    var shutdownTimes = selected.filter(function(s) { return s.shutdownTime; }).map(function(s) { return s.shutdownTime; }).sort();
    var hasActive = selected.some(function(s) { return !s.shutdownTime; });

    var merged = {
      id: genId(),
      bootTime: bootTimes[0],
      shutdownTime: hasActive ? null : (shutdownTimes.length ? shutdownTimes[shutdownTimes.length - 1] : null),
      duration: null
    };
    if (merged.bootTime && merged.shutdownTime) {
      merged.duration = new Date(merged.shutdownTime) - new Date(merged.bootTime);
    }

    var idSet = {};
    ids.forEach(function(id) { idSet[id] = true; });
    data.sessions = data.sessions.filter(function(s) { return !idSet[s.id]; });
    data.sessions.push(merged);
    data.sessions.sort(function(a, b) { return (a.bootTime || '').localeCompare(b.bootTime || ''); });
    data.bootCount = data.sessions.length;
    data.shutdownCount = data.sessions.filter(function(s) { return s.shutdownTime; }).length;

    var trashRaw = localStorage.getItem('bootTrackerTrash');
    var trash = trashRaw ? JSON.parse(trashRaw) : { sessions: [] };
    trash.sessions = trash.sessions.concat(selected);
    localStorage.setItem('bootTrackerTrash', JSON.stringify(trash));

    if (_isServerAvailable() && window._serverReady) {
      fetch(_serverUrl('/api/data'), {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(data)
      }).catch(function() {});
      fetch(_serverUrl('/api/trash'), {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(trash)
      }).catch(function() {});
    }

    saveData(data);
    var active = data.sessions.find(function(s) { return !s.shutdownTime; });
    if (active) sessionStorage.setItem('currentSessionId', active.id);
    else sessionStorage.removeItem('currentSessionId');
    window._bootDataCache = data;
    updateMainUI(data, active || null);
    refreshAdmin();
    refreshMainTable();
    refreshChart();
    renderHeatmap();
  }
}

// ——— 启动 ———
document.addEventListener('DOMContentLoaded', function() {
  _serverPort = _detectServerPort();

  // 始终先用本地缓存立即渲染（避免空白等待）
  var localData = loadData();
  if (localData.sessions && localData.sessions.length > 0) {
    _doInitBoot(localData);
  } else {
    // 显示加载状态
    var bootTimeEl = document.getElementById('currentBootTime');
    var statusEl = document.getElementById('currentStatus');
    var runTimeEl = document.getElementById('runningTime');
    if (bootTimeEl) bootTimeEl.textContent = '加载中...';
    if (statusEl) statusEl.innerHTML = '加载中...';
    if (runTimeEl) runTimeEl.textContent = '...';
  }

  // 异步从服务器加载（如果有）
  loadDataAsync(function(data) {
    _doInitBoot(data);
  });
});

// ——— 关于 & 更新 ———
function loadVersionInfo() {
  var verEl = document.getElementById('aboutVersion');
  var modeEl = document.getElementById('aboutMode');
  var resultEl = document.getElementById('updateResult');

  // 重置状态
  resultEl.textContent = '';
  resultEl.className = 'update-result';

  if (_isServerAvailable() && window._serverReady) {
    fetch(_serverUrl('/api/version'))
      .then(function(r) { return r.json(); })
      .then(function(info) {
        verEl.textContent = 'v' + info.version;
        document.getElementById('footerVersion').textContent = 'v' + info.version;
        modeEl.textContent = '\u684C\u9762\u6A21\u5F0F';
        if (!info.updateUrl) {
          resultEl.innerHTML = '<span class="no-update">\u672A\u914D\u7F6E\u8FDC\u7A0B\u66F4\u65B0\u68C0\u67E5</span>';
        }
        // 加载版本历史
        loadVersionHistory();
      })
      .catch(function() {
        verEl.textContent = 'v1.0.0';
        modeEl.textContent = '\u7EAF\u6D4F\u89C8\u5668';
      });
  } else {
    verEl.textContent = 'v1.0.0';
    modeEl.textContent = '\u7EAF\u6D4F\u89C8\u5668';
    resultEl.innerHTML = '<span class="no-update">\u4EC5\u684C\u9762\u6A21\u5F0F\u652F\u6301\u7248\u672C\u7BA1\u7406</span>';
  }
}

function loadVersionHistory() {
  fetch(_serverUrl('/api/version/history'))
    .then(function(r) { return r.json(); })
    .then(function(res) {
      var container = document.getElementById('versionHistory');
      var history = res.history || [];
      if (history.length === 0) {
        container.innerHTML = '<div style="color:#475569;font-size:0.78rem;padding:6px 0;">\u6682\u65E0\u53D1\u5E03\u5386\u53F2</div>';
        return;
      }
      var html = '';
      history.forEach(function(entry) {
        var timeStr = entry.time ? entry.time.replace('T', ' ').slice(0, 19) : '';
        html += '<div class="vh-entry">' +
          '<span class="vh-type ' + entry.type + '">' + entry.type + '</span>' +
          '<div class="vh-body">' +
            '<div class="vh-ver">v' + entry.from + ' \u2192 v' + entry.to + '</div>' +
            (entry.notes ? '<div class="vh-notes">' + entry.notes + '</div>' : '') +
          '</div>' +
          '<span class="vh-time">' + timeStr + '</span>' +
          '</div>';
      });
      container.innerHTML = html;
    })
    .catch(function() {});
}

function bumpVersion(type) {
  if (!_isServerAvailable() || !window._serverReady) {
    alert('\u4EC5\u684C\u9762\u6A21\u5F0F\u652F\u6301');
    return;
  }

  var notes = document.getElementById('bumpNotes').value.trim();
  var typeLabels = { patch: '\u8865\u4E01', minor: '\u6B21\u7248\u672C', major: '\u4E3B\u7248\u672C' };
  if (!confirm('\u786E\u5B9A\u53D1\u5E03 ' + typeLabels[type] + ' \u66F4\u65B0\uFF1F\n\u5F53\u524D\u7248\u672C v' +
    (document.getElementById('aboutVersion').textContent || '?') + ' \u5C06\u81EA\u52A8\u9012\u589E')) return;

  var msgEl = document.getElementById('bumpMsg');
  msgEl.textContent = '\u53D1\u5E03\u4E2D...';
  msgEl.className = 'form-msg';

  fetch(_serverUrl('/api/version/bump'), {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ type: type, notes: notes }),
  }).then(function(r) { return r.json(); })
    .then(function(res) {
      if (res.ok) {
        msgEl.textContent = '\u53D1\u5E03\u6210\u529F v' + res.previous + ' \u2192 v' + res.version;
        msgEl.className = 'form-msg success';
        // 更新显示
        document.getElementById('aboutVersion').textContent = 'v' + res.version;
        document.getElementById('footerVersion').textContent = 'v' + res.version;
        document.getElementById('bumpNotes').value = '';
        // 刷新历史
        loadVersionHistory();
      } else {
        msgEl.textContent = '\u53D1\u5E03\u5931\u8D25: ' + (res.error || '\u672A\u77E5');
        msgEl.className = 'form-msg error';
      }
    })
    .catch(function(err) {
      msgEl.textContent = '\u7F51\u7EDC\u9519: ' + err.message;
      msgEl.className = 'form-msg error';
    });
}

function checkUpdate() {
  var btn = document.getElementById('btnCheckUpdate');
  var resultEl = document.getElementById('updateResult');

  btn.disabled = true;
  btn.textContent = '\u68C0\u67E5\u4E2D...';
  resultEl.innerHTML = '';
  resultEl.className = 'update-result';

  if (!_isServerAvailable() || !window._serverReady) {
    resultEl.innerHTML = '<span class="error">\u4EC5\u684C\u9762\u6A21\u5F0F\u652F\u6301\u66F4\u65B0\u68C0\u67E5</span>';
    btn.disabled = false;
    btn.textContent = '\uD83D\uDD0D \u68C0\u67E5\u66F4\u65B0';
    return;
  }

  fetch(_serverUrl('/api/check-update'), { method: 'POST' })
    .then(function(r) { return r.json(); })
    .then(function(res) {
      btn.disabled = false;
      btn.textContent = '\uD83D\uDD0D \u68C0\u67E5\u66F4\u65B0';

      if (res.message) {
        resultEl.innerHTML = '<span class="no-update">' + res.message + '</span>';
      } else if (res.hasUpdate) {
        var html = '<span class="has-update">';
        html += '\u53D1\u73B0\u65B0\u7248\u672C v' + res.latest + ' \u2192 ';
        if (res.url) {
          html += '<a href="' + res.url + '" target="_blank" rel="noopener">\u4E0B\u8F7D</a>';
        } else {
          html += '\u8BF7\u624B\u52A8\u66F4\u65B0';
        }
        if (res.notes) html += ' <small>(' + res.notes + ')</small>';
        html += '</span>';
        resultEl.innerHTML = html;
      } else {
        resultEl.innerHTML = '<span class="no-update">\u5DF2\u662F\u6700\u65B0\u7248\u672C v' + res.current + '</span>';
      }
    })
    .catch(function(err) {
      btn.disabled = false;
      btn.textContent = '\uD83D\uDD0D \u68C0\u67E5\u66F4\u65B0';
      resultEl.innerHTML = '<span class="error">\u7F51\u7EDC\u9519\uFF1A' + err.message + '</span>';
    });
}

// ——— 应用设置 ———
function loadAppSettings() {
  if (!_isServerAvailable() || !window._serverReady) return;

  fetch(_serverUrl('/api/settings'))
    .then(function(r) { return r.json(); })
    .then(function(s) {
      // 通用
      document.getElementById('setAutoStart').checked = !!s.autoStart;
      document.getElementById('setAutoBackup').checked = !!s.autoBackup;
      document.getElementById('setBackupCount').value = s.backupCount || 30;
      document.getElementById('setAutoCloseIdle').checked = !!s.autoCloseIdle;
      document.getElementById('setIdleCloseMinutes').value = s.idleCloseMinutes || 5;
      // 显示
      _updateSettingOpt('defaultChartType', s.defaultChartType, 'setChartBar', 'setChartLine');
      _updateSettingOpt('timeFormat', s.timeFormat, 'setTime24', 'setTime12');
      // 自定义背景
      if (s.customBgImage) {
        applyCustomBg(s.customBgImage);
        document.getElementById('btnClearBg').style.display = '';
        document.getElementById('bgPreviewRow').style.display = '';
        document.getElementById('bgPreview').src = _serverUrl(s.customBgImage);
      }
      // 网络
      document.getElementById('setLanAccess').checked = !!s.lanAccess;
      document.getElementById('setTunnelEnabled').checked = !!s.tunnelEnabled;
      document.getElementById('setTunnelToken').value = s.tunnelToken || '';
      document.getElementById('setCustomDomain').value = s.customDomain || '';
      // 隧道开启时加载外网链接
      if (s.tunnelEnabled) {
        loadTunnelUrl();
        toggleTunnelSettings();
      } else {
        document.getElementById('tunnelUrlRow').style.display = 'none';
        toggleTunnelSettings();
      }
    })
    .catch(function() {});
}

var _tunnelUrlTimer = null;

function toggleTunnelSettings() {
  var enabled = document.getElementById('setTunnelEnabled').checked;
  var tokenRow = document.getElementById('tunnelTokenRow');
  var domainRow = document.getElementById('customDomainRow');
  var helpRow = document.getElementById('tunnelHelpRow');
  if (tokenRow) tokenRow.style.display = enabled ? '' : 'none';
  if (domainRow) domainRow.style.display = enabled ? '' : 'none';
  if (helpRow) helpRow.style.display = enabled ? '' : 'none';
}

function loadTunnelUrl() {
  document.getElementById('tunnelUrlRow').style.display = '';
  var link = document.getElementById('tunnelUrlLink');
  var desc = document.getElementById('tunnelUrlDesc');
  link.textContent = '获取中...';
  link.href = '#';

  var token = document.getElementById('setTunnelToken').value;
  var domain = document.getElementById('setCustomDomain').value;

  if (token && domain) {
    if (desc) desc.textContent = '使用自定义域名';
    link.textContent = 'https://' + domain;
    link.href = 'https://' + domain;
    return;
  }

  if (desc) desc.textContent = '每次重启会变化，点击复制';

  function fetchUrl() {
    fetch(_serverUrl('/api/tunnel-url'))
      .then(function(r) { return r.json(); })
      .then(function(res) {
        if (res.url) {
          link.textContent = res.url;
          link.href = res.url;
          if (_tunnelUrlTimer) { clearInterval(_tunnelUrlTimer); _tunnelUrlTimer = null; }
        }
      })
      .catch(function() {});
  }
  fetchUrl();
  // 每 3 秒重试，直到拿到 URL
  if (_tunnelUrlTimer) clearInterval(_tunnelUrlTimer);
  _tunnelUrlTimer = setInterval(fetchUrl, 3000);
  // 最多重试 2 分钟
  setTimeout(function() {
    if (_tunnelUrlTimer) { clearInterval(_tunnelUrlTimer); _tunnelUrlTimer = null; }
  }, 120000);
}

function copyTunnelUrl() {
  var link = document.getElementById('tunnelUrlLink');
  var url = link.href;
  if (!url || url === '#') return;
  // 在 pywebview 中 clipboardData 可能不可用，用 fallback
  if (navigator.clipboard && navigator.clipboard.writeText) {
    navigator.clipboard.writeText(url);
  } else {
    var ta = document.createElement('textarea');
    ta.value = url;
    document.body.appendChild(ta);
    ta.select();
    try { document.execCommand('copy'); } catch(e) {}
    document.body.removeChild(ta);
  }
  var orig = link.textContent;
  link.textContent = '已复制!';
  setTimeout(function() { link.textContent = orig; }, 1500);
}

function _updateSettingOpt(key, val, id1, id2) {
  var el1 = document.getElementById(id1);
  var el2 = document.getElementById(id2);
  if (el1 && el2) {
    var map = {};
    map[id1] = key === 'defaultChartType' ? 'bar' : '24h';
    map[id2] = key === 'defaultChartType' ? 'line' : '12h';
    el1.classList.toggle('active', val === map[id1]);
    el2.classList.toggle('active', val === map[id2]);
  }
}

function saveSetting(key, value) {
  if (!_isServerAvailable() || !window._serverReady) return;

  var body = {};
  body[key] = value;

  fetch(_serverUrl('/api/settings'), {
    method: 'PUT',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(body)
  })
    .then(function(r) { return r.json(); })
    .then(function(res) {
      if (!res.ok) {
        alert('\u4FDD\u5B58\u5931\u8D25: ' + (res.error || '\u672A\u77E5\u9519\u8BEF'));
        loadAppSettings(); // 回滚 UI
        return;
      }
      // 同步选项按钮状态（图表类型/时间格式）
      if (key === 'defaultChartType') _updateSettingOpt(key, value, 'setChartBar', 'setChartLine');
      if (key === 'timeFormat') _updateSettingOpt(key, value, 'setTime24', 'setTime12');
      // 备份保留数即时生效：更新 MAX_BACKUPS
      if (key === 'backupCount') D.MAX_BACKUPS = value;
      // 隧道开关：显示/隐藏外网链接行
      if (key === 'tunnelEnabled') {
        if (value) {
          setTimeout(loadTunnelUrl, 1000);
          toggleTunnelSettings();
        } else {
          document.getElementById('tunnelUrlRow').style.display = 'none';
          if (_tunnelUrlTimer) { clearInterval(_tunnelUrlTimer); _tunnelUrlTimer = null; }
          toggleTunnelSettings();
        }
      }
      // 隧道令牌或域名变化时更新显示
      if (key === 'tunnelToken' || key === 'customDomain') {
        var enabled = document.getElementById('setTunnelEnabled').checked;
        if (enabled) {
          loadTunnelUrl();
        }
      }
    })
    .catch(function(err) {
      alert('\u7F51\u7EDC\u9519\u8BEF: ' + err.message);
      loadAppSettings();
    });
}

function restartServer() {
  if (!_isServerAvailable() || !window._serverReady) return;
  var btn = document.getElementById('btnRestartServer');
  if (!btn) return;
  btn.textContent = '\u91CD\u542F\u4E2D...';
  btn.disabled = true;
  fetch(_serverUrl('/api/restart-server'), { method: 'POST' })
    .then(function(r) { return r.json(); })
    .then(function(res) {
      if (res.ok) {
        btn.textContent = '\u5DF2\u53D1\u9001';
        // 服务器正在重启，等待后刷新页面
        setTimeout(function() {
          location.reload();
        }, 1500);
      } else {
        btn.textContent = '\u91CD\u542F';
        btn.disabled = false;
        alert('\u91CD\u542F\u5931\u8D25: ' + (res.error || '\u672A\u77E5\u9519\u8BEF'));
      }
    })
    .catch(function(err) {
      btn.textContent = '\u91CD\u542F';
      btn.disabled = false;
      alert('\u7F51\u7EDC\u9519\u8BEF: ' + err.message);
    });
}

function switchSettingsTab(page, btn) {
  // 切换标签页按钮状态
  var tabs = document.querySelectorAll('.settings-tab');
  for (var i = 0; i < tabs.length; i++) {
    tabs[i].classList.remove('active');
  }
  if (btn) btn.classList.add('active');

  // 切换页面显示
  var pages = document.querySelectorAll('.settings-page');
  for (var j = 0; j < pages.length; j++) {
    if (pages[j].getAttribute('data-page') === page) {
      pages[j].classList.add('active');
    } else {
      pages[j].classList.remove('active');
    }
  }

  // 切到关于页时加载版本信息
  if (page === 'about') {
    loadVersionInfo();
  }
}

function applyCustomBg(imgPath) {
  var url = _serverUrl(imgPath) + '?t=' + Date.now();
  document.body.style.backgroundImage = 'url(' + url + ')';
  document.body.style.backgroundSize = 'cover';
  document.body.style.backgroundPosition = 'center';
  document.body.style.backgroundRepeat = 'no-repeat';
  document.body.style.animation = 'none';
  document.body.classList.add('has-custom-bg');
}

function uploadBgImage(input) {
  var file = input.files[0];
  if (!file) return;

  var formData = new FormData();
  formData.append('file', file);

  fetch(_serverUrl('/api/upload-bg'), {
    method: 'POST',
    body: formData,
  })
  .then(function(r) { return r.json(); })
  .then(function(res) {
    if (res.ok && res.path) {
      applyCustomBg(res.path);
      document.getElementById('btnClearBg').style.display = '';
      document.getElementById('bgPreviewRow').style.display = '';
      document.getElementById('bgPreview').src = _serverUrl(res.path);
    } else {
      alert('上传失败: ' + (res.error || '未知错误'));
    }
  })
  .catch(function(e) {
    alert('上传失败: ' + e.message);
  });

  input.value = '';
}

function clearBgImage() {
  document.body.style.backgroundImage = '';
  document.body.style.backgroundSize = '';
  document.body.style.backgroundPosition = '';
  document.body.style.backgroundRepeat = '';
  document.body.style.animation = '';
  document.body.classList.remove('has-custom-bg');

  document.getElementById('btnClearBg').style.display = 'none';
  document.getElementById('bgPreviewRow').style.display = 'none';

  saveSetting('customBgImage', '');
}
