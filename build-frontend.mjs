// 构建前把前端静态资源复制到 dist/，供 Tauri 打包（frontendDist = "../dist"）。
// 由 tauri.conf.json 的 beforeBuildCommand 调用。
import { cpSync, mkdirSync, existsSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = dirname(fileURLToPath(import.meta.url));
const dist = join(root, 'dist');
mkdirSync(dist, { recursive: true });

const files = ['ftue.html', 'settings.html', 'obs-overlay.html', 'design-tokens.css'];
for (const f of files) {
  const src = join(root, f);
  if (existsSync(src)) {
    cpSync(src, join(dist, f));
    console.log(`  ✓ ${f}`);
  } else {
    console.warn(`  ! 缺失 ${f}，已跳过`);
  }
}
console.log('前端已复制到 dist/');
