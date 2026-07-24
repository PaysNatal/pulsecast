/* 怦然 PulseCast · 移动端伴侣 App 逻辑
 * - 三屏导航（设备/实况/设置）
 * - 通过 WebSocket 直连本地「怦然」桌面服务拉取实时心率
 * - Web Bluetooth 扫描心率设备（HCI 0x180D / 0x2A37）
 * - 设置持久化（localStorage）+ 服务器地址可配置
 */
(function () {
  "use strict";

  const $ = (s, r = document) => r.querySelector(s);
  const $$ = (s, r = document) => Array.from(r.querySelectorAll(s));

  const state = {
    host: localStorage.getItem("pc_host") || "localhost",
    port: localStorage.getItem("pc_port") || "4567",
    token: new URLSearchParams(location.search).get("token") || localStorage.getItem("pc_token") || "",
    ws: null,
    bpm: 0,
    hiddenBpm: 72,
    status: "connecting",
    device: null,
    threshold: { high: 150, low: 55 },
    alertOn: localStorage.getItem("pc_alert") !== "false",
    theme: localStorage.getItem("pc_theme") || "dark",
    style: localStorage.getItem("pc_overlay") || "pill",
    oscOn: localStorage.getItem("pc_osc") === "true",
    autoOn: localStorage.getItem("pc_auto") !== "false",
    liveSince: null,
    ecgData: new Array(160).fill(28),
  };

  /* ───────── 导航 ───────── */
  function showScreen(name) {
    $$(".screen").forEach((s) => { s.hidden = s.dataset.screen !== name; });
    $$(".tab").forEach((t) => t.classList.toggle("active", t.dataset.go === name));
  }
  $$(".tab").forEach((t) => t.addEventListener("click", () => showScreen(t.dataset.go)));

  /* ───────── 连接状态条 ───────── */
  const statusText = { connecting: "连接中…", live: "已连接 · 实时心率", device_lost: "设备已断开", error: "连接错误" };
  function setStatus(st, msg) {
    state.status = st;
    $("#connbar").dataset.status = st;
    $("#connbar-text").textContent = msg || statusText[st] || st;
  }

  /* ───────── WebSocket 拉流 ───────── */
  function wsUrl() {
    const base = `ws://${state.host}:${state.port}/ws`;
    return state.token ? `${base}?token=${encodeURIComponent(state.token)}` : base;
  }
  function connect() {
    setStatus("connecting");
    if (state.ws) { try { state.ws.close(); } catch (e) {} }
    let ws;
    try { ws = new WebSocket(wsUrl()); }
    catch (e) { setStatus("error", "地址无效：" + wsUrl()); return; }
    state.ws = ws;
    ws.onopen = () => { setStatus("live"); };
    ws.onmessage = (ev) => {
      let f; try { f = JSON.parse(ev.data); } catch (e) { return; }
      // 控制事件（来自移动端命令的回显或桌面端/其他控制方）→ 提示条
      if (f.kind) { onControlEvent(f); return; }
      if (typeof f.bpm === "number") {
        state.bpm = f.bpm;
        if (state.bpm > 0) state.hiddenBpm = state.bpm;
      }
      if (f.status) setStatus(f.status, f.message || statusText[f.status]);
      if (f.device) state.device = f.device;
      if (f.threshold) {
        state.threshold = f.threshold;
        // 服务端阈值变化时同步控制卡显示
        if ($("#ctl-high")) $("#ctl-high").textContent = state.threshold.high;
        if ($("#ctl-low")) $("#ctl-low").textContent = state.threshold.low;
      }
      renderLive();
    };
    ws.onclose = () => setStatus("device_lost", "与桌面服务断开");
    ws.onerror = () => setStatus("error", "无法连接 " + wsUrl());
  }

  /* ───────── 屏2：实况渲染 ───────── */
  let avgArr = [];
  function renderLive() {
    const b = state.bpm > 0 ? state.bpm : state.hiddenBpm;
    $("#bpm-num").textContent = b;
    $("#th-high").textContent = state.threshold.high;
    $("#th-low").textContent = state.threshold.low;
    $("#share-ip").textContent = state.host;
    $("#share-port").textContent = state.port;
    $("#live-chip-text").textContent = state.status === "live" ? "实时直播中" : "未连接";
    if (state.status === "live" && !state.liveSince) state.liveSince = Date.now();
    if (state.liveSince) {
      const s = Math.floor((Date.now() - state.liveSince) / 1000);
      const mm = String(Math.floor(s / 60)).padStart(2, "0");
      const ss = String(s % 60).padStart(2, "0");
      $("#stat-time").textContent = `${mm}:${ss}`;
    }
    if (state.bpm > 0) {
      avgArr.push(state.bpm); if (avgArr.length > 120) avgArr.shift();
      const avg = Math.round(avgArr.reduce((a, b) => a + b, 0) / avgArr.length);
      $("#stat-avg").textContent = avg;
      $("#stat-max").textContent = Math.max(...avgArr);
    }
  }

  /* ───────── ECG 动画 ───────── */
  const canvas = $("#ecg");
  const ctx = canvas.getContext("2d");
  const W = canvas.width, H = canvas.height, mid = H / 2;
  let lastBeat = 0;
  function pushSample() {
    const b = state.bpm > 0 ? state.bpm : state.hiddenBpm;
    const beatsPerSec = b / 60;
    const now = performance.now();
    const dx = (now - lastBeat) / 1000;
    let y = mid;
    // 每拍绘制一个 ECG 复合波（P-QRS-T 简化）
    const phase = (dx * beatsPerSec) % 1;
    if (phase < 0.12) y = mid - 6;                       // P
    else if (phase < 0.16) y = mid + 4;                  // Q
    else if (phase < 0.19) y = mid - 26;                 // R 尖峰
    else if (phase < 0.23) y = mid + 12;                 // S
    else if (phase < 0.42) y = mid - 8;                  // T
    else y = mid;
    state.ecgData.push(y); state.ecgData.shift();
  }
  function drawEcg() {
    ctx.clearRect(0, 0, W, H);
    ctx.strokeStyle = getCss("--accent"); ctx.lineWidth = 2.4;
    ctx.lineJoin = "round"; ctx.lineCap = "round";
    ctx.beginPath();
    const n = state.ecgData.length;
    for (let i = 0; i < n; i++) {
      const x = (i / (n - 1)) * W;
      const y = state.ecgData[i];
      if (i === 0) ctx.moveTo(x, y); else ctx.lineTo(x, y);
    }
    ctx.stroke();
  }
  function ecgLoop() { pushSample(); drawEcg(); ecgRafId = requestAnimationFrame(ecgLoop); }
  var ecgRafId = null;
  function startEcg() { if (!ecgRafId) ecgRafId = requestAnimationFrame(ecgLoop); }
  function stopEcg() { if (ecgRafId) { cancelAnimationFrame(ecgRafId); ecgRafId = null; } }
  document.addEventListener("visibilitychange", function() {
    if (document.hidden) stopEcg(); else startEcg();
  });
  function getCss(v) { return getComputedStyle(document.documentElement).getPropertyValue(v).trim() || "#FF4D6D"; }

  /* ───────── 屏1：设备扫描（Web Bluetooth） ───────── */
  const KNOWN = [
    { id: "amazfit", name: "Amazfit GTR 4", icon: "⌚", known: true },
    { id: "miband", name: "小米手环 8 Pro", icon: "⌚", known: true },
    { id: "huawei", name: "华为 Watch GT 4", icon: "⌚", known: true },
    { id: "polar", name: "Polar H10 胸带", icon: "❤", known: true },
  ];
  function renderDevices(devices) {
    const list = $("#device-list"); list.innerHTML = "";
    devices.forEach((d) => {
      const connected = state.device && state.device.name === d.name;
      const row = document.createElement("div");
      row.className = "device" + (connected ? " connected" : "");
      row.innerHTML =
        `<div class="badge">${d.icon}</div>` +
        `<div class="info"><div class="name">${d.name}</div>` +
        `<div class="meta ${connected ? "ok" : ""}">${connected ? "已连接 · 实时心率" : "本地蓝牙直连"}</div></div>` +
        `<button class="pill ${connected ? "is-connected" : "outline"}">${connected ? "已连接" : "连接"}</button>`;
      list.appendChild(row);
    });
  }
  async function scanBle() {
    $("#scan-title").textContent = "正在扫描附近设备…";
    if (!("bluetooth" in navigator)) {
      // 不支持 Web Bluetooth（如 iOS Safari / 多数安卓 WebView）→ 回退已知设备清单
      $("#scan-sub").textContent = "当前环境不支持 Web 蓝牙，展示兼容设备清单";
      renderDevices(KNOWN);
      return;
    }
    try {
      const device = await navigator.bluetooth.requestDevice({
        filters: [{ services: ["heart_rate"] }],
        optionalServices: ["heart_rate"],
      });
      const d = { id: device.id, name: device.name || "未知设备", icon: "❤", known: false };
      $("#scan-sub").textContent = "已发现 1 台 · 点击连接";
      renderDevices([d, ...KNOWN]);
    } catch (e) {
      $("#scan-sub").textContent = "扫描已取消或失败，展示兼容设备清单";
      renderDevices(KNOWN);
    }
  }

  /* ───────── 设置交互 ───────── */
  function bindToggle(el, key, onSync) {
    el.addEventListener("click", () => {
      const on = el.getAttribute("aria-checked") !== "true";
      el.setAttribute("aria-checked", String(on));
      localStorage.setItem(key, String(on));
      state[key.replace("pc_", "")] = on;
      if (onSync) onSync(on);
    });
  }
  bindToggle($("#tg-alert"), "pc_alert");
  bindToggle($("#tg-alert2"), "pc_alert");
  bindToggle($("#tg-osc"), "pc_osc");
  bindToggle($("#tg-auto"), "pc_auto");
  // 双向同步两个提醒开关
  function syncAlert(v) { [$("#tg-alert"), $("#tg-alert2")].forEach((t) => t.setAttribute("aria-checked", String(v))); }
  $("#tg-alert").addEventListener("click", () => syncAlert($("#tg-alert").getAttribute("aria-checked") === "true"));
  $("#tg-alert2").addEventListener("click", () => syncAlert($("#tg-alert2").getAttribute("aria-checked") === "true"));

  function bindSegment(wrap, key, cb) {
    $$( "button", wrap).forEach((b) => {
      b.addEventListener("click", () => {
        $$( "button", wrap).forEach((x) => x.classList.remove("active"));
        b.classList.add("active");
        localStorage.setItem(key, b.dataset.v);
        if (cb) cb(b.dataset.v);
      });
    });
  }
  bindSegment($("#seg-theme"), "pc_theme");
  bindSegment($("#style-row"), "pc_overlay");

  // 初始化设置 UI 状态
  function initSettingsUI() {
    $("#tg-alert").setAttribute("aria-checked", String(state.alertOn));
    $("#tg-alert2").setAttribute("aria-checked", String(state.alertOn));
    $("#tg-osc").setAttribute("aria-checked", String(state.oscOn));
    $("#tg-auto").setAttribute("aria-checked", String(state.autoOn));
    $$( '#seg-theme button' ).forEach((b) => b.classList.toggle("active", b.dataset.v === state.theme));
    $$( '#style-row button' ).forEach((b) => b.classList.toggle("active", b.dataset.v === state.style));
    $("#set-port").textContent = state.port;
    $("#cfg-host").value = state.host;
    $("#cfg-port").value = state.port;
    $("#cfg-token").value = state.token;
  }

  /* ───────── 服务器配置弹层 ───────── */
  $("#btn-config").addEventListener("click", () => { $("#cfg-modal").hidden = false; });
  $("#cfg-cancel").addEventListener("click", () => { $("#cfg-modal").hidden = true; });
  $("#cfg-save").addEventListener("click", () => {
    const h = $("#cfg-host").value.trim() || "localhost";
    const p = $("#cfg-port").value.trim() || "4567";
    const tk = $("#cfg-token").value.trim();
    state.host = h; state.port = p; state.token = tk;
    localStorage.setItem("pc_host", h); localStorage.setItem("pc_port", p);
    localStorage.setItem("pc_token", tk);
    $("#set-port").textContent = p;
    $("#cfg-modal").hidden = true;
    connect();
  });
  $("#btn-share").addEventListener("click", async () => {
    const text = `ws://${state.host}:${state.port}`;
    try { await navigator.clipboard.writeText(text); $("#btn-share").textContent = "已复制"; setTimeout(() => ($("#btn-share").textContent = "分享"), 1500); }
    catch (e) { alert("串流地址：" + text); }
  });

  /* ───────── 控制中心：手机指挥桌面 OBS ───────── */
  function sendCommand(kind, payload) {
    if (!state.ws || state.ws.readyState !== 1) { flashToast("未连接桌面端"); return; }
    const msg = Object.assign({ kind }, payload || {});
    state.ws.send(JSON.stringify(msg));
  }
  // 场景切换 chips
  $$(".scene-chip").forEach((b) =>
    b.addEventListener("click", () => sendCommand("switch_scene", { scene: b.dataset.scene }))
  );
  // 高光闪烁
  $("#btn-flash").addEventListener("click", () => sendCommand("flash"));
  // 实时阈值步进
  function stepThr(which, d) {
    if (which === "high") state.threshold.high = Math.min(220, state.threshold.high + d * 5);
    else state.threshold.low = Math.max(40, state.threshold.low + d * 5);
    $("#ctl-high").textContent = state.threshold.high;
    $("#ctl-low").textContent = state.threshold.low;
  }
  $$(".thr-plus").forEach((b) => b.addEventListener("click", () => stepThr(b.dataset.thr, +1)));
  $$(".thr-minus").forEach((b) => b.addEventListener("click", () => stepThr(b.dataset.thr, -1)));
  $("#btn-send-thr").addEventListener("click", () => {
    if (state.threshold.low >= state.threshold.high) { flashToast("低阈值须小于高阈值"); return; }
    sendCommand("set_threshold", { high: state.threshold.high, low: state.threshold.low });
  });
  // 远端控制事件提示
  let toastTimer = null;
  function flashToast(txt) {
    const t = $("#ctl-toast");
    if (!t) return;
    t.textContent = txt;
    t.hidden = false;
    requestAnimationFrame(() => t.classList.add("show"));
    clearTimeout(toastTimer);
    toastTimer = setTimeout(() => {
      t.classList.remove("show");
      setTimeout(() => { t.hidden = true; }, 250);
    }, 1800);
  }
  function onControlEvent(f) {
    let txt = "";
    if (f.kind === "switch_scene") txt = "切换场景 → " + (f.scene || "?");
    else if (f.kind === "flash") txt = "✨ 高光闪烁！";
    else if (f.kind === "set_threshold") txt = "阈值已更新 " + f.low + "–" + f.high;
    else if (f.kind === "reset") txt = "已复位";
    if (txt) flashToast(txt);
  }

  /* ───────── 启动 ───────── */
  initSettingsUI();
  renderDevices(KNOWN);
  connect();
  startEcg();
  // 首次进入先扫描（若支持 Web 蓝牙）
  setTimeout(scanBle, 400);
  // 断线自动重连
  setInterval(() => { if (state.ws && state.ws.readyState > 1) connect(); }, 5000);
})();
