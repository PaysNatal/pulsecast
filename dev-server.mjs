// 怦然 PulseCast · 本地服务（开发态）
// 职责：托管 ftue / settings / obs 三页 + 通过 ws://localhost:4567 推送实时心率。
// 注意：这是「参考实现」。生产环境由 Tauri 的 Rust 后端（btleplug 读 BLE + 内嵌 HTTP/WS）
// 提供相同契约的本地服务，OBS 浏览器源与桌面前端都连同一个地址。
import http from "node:http";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { WebSocketServer } from "ws";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const PORT = Number(process.env.PORT) || 4567;

// 路由：根路径 = FTUE 向导，桌面窗口也加载它
const ROUTES = {
  "/": "ftue.html",
  "/settings": "settings.html",
  "/obs": "obs-overlay.html",
};

const MIME = {
  ".html": "text/html; charset=utf-8",
  ".css": "text/css; charset=utf-8",
  ".js": "text/javascript; charset=utf-8",
  ".svg": "image/svg+xml",
};

// ── 心率数据源（模拟；真实环境由 Rust 侧 BLE 服务写入 state） ──
const state = {
  bpm: 72,
  status: "live",
  device: { name: "Amazfit GTR 4", type: "watch", signal: "strong" },
  message: "",
};

function tick() {
  // 68–78 轻微波动，模拟真实心率
  state.bpm = 68 + Math.round(Math.random() * 10);
}

// ── HTTP 静态服务 ──
const server = http.createServer((req, res) => {
  const urlPath = req.url.split("?")[0];
  let file = ROUTES[urlPath];
  if (!file) {
    if (urlPath === "/obs-overlay.html") file = "obs-overlay.html";
    else if (urlPath === "/settings.html") file = "settings.html";
    else if (urlPath === "/ftue.html") file = "ftue.html";
    else if (urlPath === "/design-tokens.css") file = "design-tokens.css";
  }
  if (!file) {
    res.writeHead(404);
    res.end("Not found");
    return;
  }
  const fp = path.join(__dirname, file);
  fs.readFile(fp, (err, data) => {
    if (err) {
      res.writeHead(404);
      res.end("Not found");
      return;
    }
    const ext = path.extname(fp);
    res.writeHead(200, { "Content-Type": MIME[ext] || "application/octet-stream" });
    res.end(data);
  });
});

// ── WebSocket：实时心率 ──
const wss = new WebSocketServer({ server });
function broadcast() {
  const payload = JSON.stringify(state);
  wss.clients.forEach((c) => {
    if (c.readyState === 1) c.send(payload);
  });
}
wss.on("connection", (ws) => {
  ws.send(JSON.stringify(state)); // 连接即推当前状态
});

setInterval(() => {
  tick();
  broadcast();
}, 1000);

server.listen(PORT, () => {
  console.log(`怦然 PulseCast 本地服务已启动：http://localhost:${PORT}`);
  console.log(`  FTUE  : http://localhost:${PORT}/`);
  console.log(`  设置  : http://localhost:${PORT}/settings`);
  console.log(`  OBS源 : http://localhost:${PORT}/obs   （OBS 浏览器源填此地址）`);
  console.log(`  WS    : ws://localhost:${PORT}`);
});
