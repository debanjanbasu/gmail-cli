import { existsSync, mkdirSync, rmSync, writeFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const scriptDirectory = dirname(fileURLToPath(import.meta.url));
const siteDirectory = resolve(scriptDirectory, '..');
const assetDirectory = resolve(siteDirectory, 'src', 'assets');
const publicDirectory = resolve(siteDirectory, 'public');

const PALETTE_LEGEND = Object.freeze({
  '.': null,
  o: '#3B1D0B',
  s: '#7C2D12',
  d: '#C2410C',
  m: '#EA580C',
  b: '#F97316',
  p: '#FF8A3D',
  w: '#FFD9B0',
  h: '#FFF3E4',
  k: '#1A120B',
  r: '#EA4335',
  g: '#34A853',
  y: '#FBBC04',
  u: '#4285F4',
  c: '#FFF8F0'
});

const xml = (value) => String(value)
  .replaceAll('&', '&amp;')
  .replaceAll('<', '&lt;')
  .replaceAll('>', '&gt;')
  .replaceAll('"', '&quot;');

const number = (value) => Number.isFinite(value) ? String(value) : '0';

const rect = (x, y, width, height, fill, extra = '') =>
  `<rect x="${number(x)}" y="${number(y)}" width="${number(width)}" height="${number(height)}" fill="${fill}"${extra ? ` ${extra}` : ''}/>`;

const polygon = (points, fill, extra = '') =>
  `<polygon points="${points.map(([x, y]) => `${number(x)},${number(y)}`).join(' ')}" fill="${fill}"${extra ? ` ${extra}` : ''}/>`;

const path = (d, fill = 'none', extra = '') =>
  `<path d="${d}" fill="${fill}"${extra ? ` ${extra}` : ''}/>`;

const line = (x1, y1, x2, y2, stroke, width = 4, extra = '') =>
  path(`M${number(x1)} ${number(y1)}L${number(x2)} ${number(y2)}`, 'none', `stroke="${stroke}" stroke-width="${number(width)}" stroke-linecap="square" stroke-linejoin="miter"${extra ? ` ${extra}` : ''}`);

const svgDocument = ({ width, height, viewBox = `0 0 ${width} ${height}`, title, description, body }) =>
  `<svg xmlns="http://www.w3.org/2000/svg" width="${number(width)}" height="${number(height)}" viewBox="${viewBox}" role="img" aria-labelledby="title desc"><title id="title">${xml(title)}</title><desc id="desc">${xml(description)}</desc>${body}\n</svg>\n`;

const snap4 = (value) => Math.round(value / 4) * 4;

const contactShadow = (cx, cy, rx, ry) => {
  const centerX = snap4(cx);
  const centerY = snap4(cy);
  const radiusX = snap4(rx);
  const radiusY = snap4(ry);
  return `<g aria-hidden="true"><ellipse cx="${number(centerX)}" cy="${number(centerY)}" rx="${number(radiusX)}" ry="${number(radiusY)}" fill="#3B1D0B" opacity=".05"/><ellipse cx="${number(centerX)}" cy="${number(snap4(cy - 4))}" rx="${number(snap4(rx - 16))}" ry="${number(snap4(ry - 4))}" fill="#3B1D0B" opacity=".06"/><ellipse cx="${number(centerX)}" cy="${number(snap4(cy - 8))}" rx="${number(snap4(rx - 36))}" ry="${number(snap4(ry - 8))}" fill="#3B1D0B" opacity=".07"/></g>`;
};

const parseAscii = (name, source) => {
  const rows = source.trim().split(/\r?\n/).map((row) => [...row]);
  if (rows.length !== 64 || rows.some((row) => row.length !== 64)) {
    throw new Error(`${name} must be a 64x64 ASCII map`);
  }
  for (const row of rows) {
    for (const cell of row) {
      if (!(cell in PALETTE_LEGEND)) {
        throw new Error(`${name} contains unknown palette cell ${cell}`);
      }
    }
  }
  return rows;
};

const blankMap = () => Array.from({ length: 64 }, () => Array(64).fill('.'));

const setPixel = (map, x, y, color) => {
  const px = Math.round(x);
  const py = Math.round(y);
  if (px >= 0 && py >= 0 && px < 64 && py < 64) map[py][px] = color;
};

const fillRect = (map, x, y, width, height, color) => {
  for (let py = y; py < y + height; py += 1) {
    for (let px = x; px < x + width; px += 1) setPixel(map, px, py, color);
  }
};

const fillEllipse = (map, cx, cy, rx, ry, color) => {
  for (let y = Math.floor(cy - ry); y <= Math.ceil(cy + ry); y += 1) {
    for (let x = Math.floor(cx - rx); x <= Math.ceil(cx + rx); x += 1) {
      const dx = (x + 0.5 - cx) / (rx + 0.5);
      const dy = (y + 0.5 - cy) / (ry + 0.5);
      if (dx * dx + dy * dy <= 1) setPixel(map, x, y, color);
    }
  }
};

const pointInPolygon = (x, y, points) => {
  let inside = false;
  for (let index = 0, previous = points.length - 1; index < points.length; previous = index, index += 1) {
    const [xi, yi] = points[index];
    const [xj, yj] = points[previous];
    const intersects = ((yi > y) !== (yj > y)) && (x < (xj - xi) * (y - yi) / ((yj - yi) || 1) + xi);
    if (intersects) inside = !inside;
  }
  return inside;
};

const fillPolygon = (map, points, color) => {
  const xs = points.map(([x]) => x);
  const ys = points.map(([, y]) => y);
  for (let y = Math.floor(Math.min(...ys)); y <= Math.ceil(Math.max(...ys)); y += 1) {
    for (let x = Math.floor(Math.min(...xs)); x <= Math.ceil(Math.max(...xs)); x += 1) {
      if (pointInPolygon(x + 0.5, y + 0.5, points)) setPixel(map, x, y, color);
    }
  }
};

const outlinePolygon = (map, points, color = 'o') => {
  fillPolygon(map, points, color);
  const xs = points.map(([x]) => x);
  const ys = points.map(([, y]) => y);
  for (let y = Math.floor(Math.min(...ys)); y <= Math.ceil(Math.max(...ys)); y += 1) {
    for (let x = Math.floor(Math.min(...xs)); x <= Math.ceil(Math.max(...xs)); x += 1) {
      if (map[y]?.[x] !== color) continue;
      const neighbors = [[x - 1, y], [x + 1, y], [x, y - 1], [x, y + 1]];
      if (neighbors.some(([nx, ny]) => map[ny]?.[nx] !== color)) continue;
      setPixel(map, x, y, '.');
    }
  }
};

const drawLine = (map, x1, y1, x2, y2, width, color) => {
  const steps = Math.max(Math.abs(x2 - x1), Math.abs(y2 - y1));
  for (let index = 0; index <= steps; index += 1) {
    const x = Math.round(x1 + (x2 - x1) * index / steps);
    const y = Math.round(y1 + (y2 - y1) * index / steps);
    fillRect(map, x - Math.floor((width - 1) / 2), y - Math.floor((width - 1) / 2), width, width, color);
  }
};

const drawLimb = (map, points, fill = 'b') => {
  for (let index = 1; index < points.length; index += 1) {
    const [x1, y1] = points[index - 1];
    const [x2, y2] = points[index];
    drawLine(map, x1, y1, x2, y2, 3, 'o');
  }
  for (let index = 1; index < points.length; index += 1) {
    const [x1, y1] = points[index - 1];
    const [x2, y2] = points[index];
    drawLine(map, x1, y1, x2, y2, 1, fill);
  }
};

const drawFoot = (map, x, y, direction, fill = 'b') => {
  const width = direction < 0 ? 6 : 6;
  const start = direction < 0 ? x - width + 1 : x;
  fillRect(map, start, y, width, 3, 'o');
  fillRect(map, start + 1, y, width - 2, 1, 'w');
  fillRect(map, start + 1, y + 1, width - 2, 1, fill);
};

const drawClaw = (map, centerX, centerY, side = 'left', open = false) => {
  const direction = side === 'left' ? -1 : 1;
  const mirror = (points) => points.map(([x, y]) => [2 * centerX - x, y]);
  const rightPoints = open
    ? [
        [centerX - 5, centerY - 5], [centerX + 3, centerY - 7],
        [centerX + 7, centerY - 4], [centerX + 6, centerY],
        [centerX + 3, centerY + 1], [centerX + 5, centerY + 4],
        [centerX, centerY + 7], [centerX - 5, centerY + 4]
      ]
    : [
        [centerX - 5, centerY - 5], [centerX + 3, centerY - 6],
        [centerX + 6, centerY - 2], [centerX + 5, centerY + 3],
        [centerX, centerY + 6], [centerX - 5, centerY + 3]
      ];
  const innerRight = open
    ? [
        [centerX - 4, centerY - 4], [centerX + 2, centerY - 6],
        [centerX + 5, centerY - 3], [centerX + 4, centerY],
        [centerX + 2, centerY + 1], [centerX + 4, centerY + 3],
        [centerX, centerY + 5], [centerX - 4, centerY + 3]
      ]
    : [
        [centerX - 4, centerY - 4], [centerX + 2, centerY - 5],
        [centerX + 4, centerY - 2], [centerX + 4, centerY + 2],
        [centerX, centerY + 4], [centerX - 4, centerY + 2]
      ];
  const points = direction < 0 ? mirror(rightPoints) : rightPoints;
  const inner = direction < 0 ? mirror(innerRight) : innerRight;
  outlinePolygon(map, points, 'o');
  fillPolygon(map, inner, 'm');
  fillRect(map, centerX - 3, centerY - 4, 5, 3, 'p');
  fillRect(map, centerX - 2, centerY - 4, 3, 1, 'h');
  setPixel(map, centerX + direction * 3, centerY + 2, 'd');
  setPixel(map, centerX - direction * 3, centerY - 1, 'b');
  if (open) {
    fillRect(map, centerX + direction * 3, centerY - 1, direction > 0 ? 1 : 2, 2, 'o');
  }
};

const drawEye = (map, x, y) => {
  fillRect(map, x, y, 10, 12, 'o');
  fillRect(map, x + 1, y + 1, 8, 10, 'h');
  fillRect(map, x + 1, y + 1, 6, 3, 'w');
  fillRect(map, x + 3, y + 4, 4, 5, 'k');
  fillRect(map, x + 3, y + 4, 2, 2, 'h');
  setPixel(map, x + 6, y + 8, 'd');
  setPixel(map, x + 1, y + 9, 'p');
};

const drawSmile = (map, open = false) => {
  if (open) {
    fillRect(map, 30, 36, 5, 4, 'o');
    fillRect(map, 31, 37, 3, 2, 'r');
    setPixel(map, 30, 39, 'd');
    setPixel(map, 34, 39, 'd');
  } else {
    fillRect(map, 28, 36, 1, 1, 'k');
    fillRect(map, 29, 37, 2, 1, 'k');
    fillRect(map, 31, 37, 2, 1, 'k');
    fillRect(map, 33, 37, 1, 1, 'k');
    fillRect(map, 34, 36, 1, 1, 'k');
  }
};

const paintBody = (map, sleeping = false) => {
  if (sleeping) {
    fillEllipse(map, 32, 37, 23, 10, 'o');
    fillEllipse(map, 32, 37, 21.5, 8.5, 'm');
    fillEllipse(map, 29, 34, 18, 6, 'b');
    fillEllipse(map, 26, 31, 11, 4, 'p');
    fillEllipse(map, 22, 28, 5, 2, 'w');
    for (let y = 29; y < 47; y += 1) {
      for (let x = 10; x < 55; x += 1) {
        const color = map[y][x];
        if (!'mbpw'.includes(color)) continue;
        if ((x > 46 && y > 34) || (y > 42 && x > 34)) map[y][x] = 'd';
        if (x > 50 && y > 40) map[y][x] = 's';
      }
    }
    return;
  }
  fillEllipse(map, 32, 35, 18, 14, 'o');
  fillEllipse(map, 32, 35, 16.6, 12.6, 'm');
  fillEllipse(map, 30, 33, 15, 10, 'b');
  fillEllipse(map, 27, 29, 12, 7, 'p');
  fillEllipse(map, 24, 26, 7, 4, 'w');
  fillEllipse(map, 21.5, 24, 2.5, 1.5, 'h');
  for (let y = 22; y < 49; y += 1) {
    for (let x = 14; x < 51; x += 1) {
      const color = map[y][x];
      if (!'mbpwh'.includes(color)) continue;
      if (x > 43 && y > 31) map[y][x] = 'd';
      if (x > 47 && y > 39) map[y][x] = 's';
      if (y > 44 && x > 30) map[y][x] = 'd';
      if (x < 20 && y < 31) map[y][x] = color === 'w' ? 'w' : 'p';
    }
  }
  fillRect(map, 36, 26, 5, 2, 'w');
  fillRect(map, 40, 28, 2, 4, 'w');
  for (const [x, y, color] of [[23, 31, 'd'], [26, 32, 'm'], [39, 33, 'd'], [42, 31, 'm'], [20, 36, 'p'], [45, 38, 'd']]) {
    fillRect(map, x, y, 2, 1, color);
  }
};

const drawSleepMark = (map, x, y, scale, color = 'k') => {
  const glyph = [
    '11111',
    '00001',
    '00001',
    '00100',
    '01000',
    '10000',
    '11111'
  ];
  glyph.forEach((row, rowIndex) => [...row].forEach((cell, columnIndex) => {
    if (cell === '1') fillRect(map, x + columnIndex * scale, y + rowIndex * scale, scale, scale, color);
  }));
};

const buildIdleMap = () => {
  const map = blankMap();
  drawLimb(map, [[21, 44], [18, 48], [15, 52]], 'b');
  drawLimb(map, [[22, 44], [25, 48], [28, 52]], 'p');
  drawLimb(map, [[43, 44], [46, 48], [49, 52]], 'p');
  drawLimb(map, [[42, 44], [39, 48], [36, 52]], 'b');
  drawFoot(map, 15, 52, -1, 'b');
  drawFoot(map, 28, 52, 1, 'p');
  drawFoot(map, 49, 52, 1, 'p');
  drawFoot(map, 36, 52, -1, 'b');
  drawLimb(map, [[19, 34], [14, 34], [11, 30]], 'b');
  drawLimb(map, [[45, 34], [50, 34], [53, 30]], 'p');
  drawClaw(map, 8, 29, 'left');
  drawClaw(map, 56, 29, 'right');
  paintBody(map);
  fillRect(map, 23, 22, 7, 4, 'p');
  fillRect(map, 34, 22, 7, 4, 'p');
  drawEye(map, 21, 12);
  drawEye(map, 33, 12);
  drawSmile(map);
  fillRect(map, 20, 35, 4, 2, 'r');
  fillRect(map, 40, 35, 4, 2, 'r');
  fillRect(map, 20, 36, 2, 1, 'p');
  fillRect(map, 42, 36, 2, 1, 'd');
  return map;
};

const buildWaveMap = () => {
  const map = blankMap();
  drawLimb(map, [[21, 44], [18, 48], [15, 52]], 'b');
  drawLimb(map, [[22, 44], [25, 48], [28, 52]], 'p');
  drawLimb(map, [[43, 44], [46, 48], [49, 52]], 'p');
  drawLimb(map, [[42, 44], [39, 48], [36, 52]], 'b');
  drawFoot(map, 15, 52, -1, 'b');
  drawFoot(map, 28, 52, 1, 'p');
  drawFoot(map, 49, 52, 1, 'p');
  drawFoot(map, 36, 52, -1, 'b');
  drawLimb(map, [[19, 34], [14, 34], [11, 30]], 'b');
  drawClaw(map, 8, 29, 'left');
  drawLimb(map, [[45, 33], [50, 27], [52, 15]], 'p');
  drawClaw(map, 54, 10, 'right', true);
  paintBody(map);
  fillRect(map, 23, 22, 7, 4, 'p');
  fillRect(map, 34, 22, 7, 4, 'p');
  drawEye(map, 21, 12);
  drawEye(map, 33, 12);
  drawSmile(map);
  fillRect(map, 20, 35, 4, 2, 'r');
  fillRect(map, 40, 35, 4, 2, 'r');
  fillRect(map, 43, 30, 3, 1, 'w');
  return map;
};

const buildSleepMap = () => {
  const map = blankMap();
  drawLimb(map, [[15, 39], [10, 41], [6, 45]], 'b');
  drawLimb(map, [[49, 39], [54, 41], [58, 45]], 'p');
  drawFoot(map, 6, 45, -1, 'b');
  drawFoot(map, 58, 45, 1, 'p');
  paintBody(map, true);
  fillRect(map, 23, 31, 7, 1, 'k');
  fillRect(map, 25, 32, 3, 1, 'k');
  fillRect(map, 34, 31, 7, 1, 'k');
  fillRect(map, 36, 32, 3, 1, 'k');
  drawSmile(map);
  fillRect(map, 20, 35, 4, 2, 'r');
  fillRect(map, 40, 35, 4, 2, 'r');
  drawSleepMark(map, 43, 24, 1, 'k');
  drawSleepMark(map, 50, 17, 1, 'k');
  drawSleepMark(map, 56, 10, 1, 'k');
  return map;
};

const buildCelebrateMap = () => {
  const map = blankMap();
  drawLimb(map, [[21, 44], [18, 48], [15, 52]], 'b');
  drawLimb(map, [[22, 44], [25, 48], [28, 52]], 'p');
  drawLimb(map, [[43, 44], [46, 48], [49, 52]], 'p');
  drawLimb(map, [[42, 44], [39, 48], [36, 52]], 'b');
  drawFoot(map, 15, 52, -1, 'b');
  drawFoot(map, 28, 52, 1, 'p');
  drawFoot(map, 49, 52, 1, 'p');
  drawFoot(map, 36, 52, -1, 'b');
  drawLimb(map, [[19, 33], [15, 25], [13, 15]], 'b');
  drawLimb(map, [[45, 33], [49, 25], [51, 15]], 'p');
  drawClaw(map, 11, 10, 'left', true);
  drawClaw(map, 53, 10, 'right', true);
  paintBody(map);
  fillRect(map, 23, 22, 7, 4, 'p');
  fillRect(map, 34, 22, 7, 4, 'p');
  drawEye(map, 21, 12);
  drawEye(map, 33, 12);
  drawSmile(map, true);
  fillRect(map, 20, 35, 4, 2, 'r');
  fillRect(map, 40, 35, 4, 2, 'r');
  const confetti = [[4, 5, 'w'], [8, 3, 'h'], [17, 3, 'p'], [25, 1, 'd'], [32, 2, 'w'], [42, 3, 'p'], [55, 4, 'd'], [59, 11, 'h'], [4, 18, 'p'], [60, 22, 'w']];
  for (const [x, y, color] of confetti) {
    fillRect(map, x, y, 2, 2, 'o');
    fillRect(map, x, y, 1, 1, color);
  }
  return map;
};

const mapRuns = (map) => {
  const size = map.length;
  const rectangles = new Map();
  let previous = [];
  for (let y = 0; y < size; y += 1) {
    const current = [];
    let x = 0;
    while (x < size) {
      const color = map[y][x];
      if (color === '.') {
        x += 1;
        continue;
      }
      const start = x;
      while (x < size && map[y][x] === color) x += 1;
      const width = x - start;
      const prior = previous.find((run) => run.color === color && run.x === start && run.width === width);
      if (prior) {
        prior.height += 1;
        current.push(prior);
      } else {
        const run = { color, x: start, y, width, height: 1 };
        rectangles.set(run, run);
        current.push(run);
      }
    }
    previous = current;
  }
  return [...rectangles.values()];
};

const mapLayer = (map, { x = 0, y = 0, scale = 8 } = {}) => {
  const runs = mapRuns(map);
  return ['o', 's', 'd', 'm', 'b', 'p', 'w', 'h', 'k', 'r', 'g', 'y', 'u'].map((color) => {
    const paths = runs.filter((run) => run.color === color).map((run) => `M${number(run.x * scale + x)} ${number(run.y * scale + y)}h${number(run.width * scale)}v${number(run.height * scale)}h-${number(run.width * scale)}z`);
    return paths.length ? `<path fill="${PALETTE_LEGEND[color]}" d="${paths.join('')}"/>` : '';
  }).join('');
};

const mascotSvg = (map, name, description) => svgDocument({
  width: 512,
  height: 512,
  viewBox: '0 0 512 512',
  title: `grr mascot ${name}`,
  description,
  body: `${contactShadow(256, 480, 112, 14)}<g shape-rendering="crispEdges">${mapLayer(map)}</g>`
});

const terminalCrab = (map, x, y, scale) => `<g shape-rendering="crispEdges">${mapLayer(map, { x, y, scale })}</g>`;

const heroTerminal = () => {
  const idle = ASCII_MAPS.idle;
  const body = `${contactShadow(480, 568, 348, 28)}<g shape-rendering="crispEdges">${polygon([[96, 448], [184, 584], [832, 504], [856, 192], [760, 96], [128, 160]], PALETTE_LEGEND.o)}${polygon([[112, 448], [192, 568], [816, 496], [840, 200], [752, 112], [144, 168]], PALETTE_LEGEND.d)}${polygon([[128, 160], [760, 96], [864, 200], [232, 280]], PALETTE_LEGEND.m)}${polygon([[144, 160], [752, 104], [824, 176], [232, 248]], PALETTE_LEGEND.b)}${polygon([[160, 160], [744, 112], [768, 132], [208, 184]], PALETTE_LEGEND.p)}${polygon([[96, 448], [128, 160], [232, 280], [192, 568]], PALETTE_LEGEND.s)}${polygon([[112, 432], [144, 192], [208, 288], [184, 520]], PALETTE_LEGEND.d)}${polygon([[232, 280], [864, 200], [832, 504], [192, 584]], PALETTE_LEGEND.o)}${polygon([[248, 288], [840, 216], [812, 480], [216, 544]], PALETTE_LEGEND.c)}${polygon([[256, 300], [824, 228], [800, 468], [232, 532]], PALETTE_LEGEND.p, 'opacity=".14"')}${polygon([[248, 288], [840, 216], [832, 248], [256, 320]], PALETTE_LEGEND.h)}${polygon([[264, 312], [792, 248], [768, 448], [240, 512]], PALETTE_LEGEND.k)}${polygon([[280, 328], [776, 268], [760, 420], [264, 480]], PALETTE_LEGEND.k)}${polygon([[288, 336], [768, 280], [760, 332], [288, 388]], PALETTE_LEGEND.s, 'opacity=".5"')}${rect(320, 352, 32, 24, PALETTE_LEGEND.p)}${rect(376, 344, 128, 16, PALETTE_LEGEND.w)}${rect(376, 376, 200, 16, PALETTE_LEGEND.d)}${rect(320, 408, 176, 16, PALETTE_LEGEND.b)}${rect(536, 400, 160, 16, PALETTE_LEGEND.m)}${rect(320, 440, 104, 16, PALETTE_LEGEND.h)}${rect(456, 432, 208, 16, PALETTE_LEGEND.s)}${rect(696, 440, 32, 24, PALETTE_LEGEND.w)}${rect(224, 536, 48, 16, PALETTE_LEGEND.d)}${rect(712, 464, 64, 16, PALETTE_LEGEND.p)}${rect(624, 472, 56, 16, PALETTE_LEGEND.b)}${terminalCrab(idle, 664, 0, 3)}${rect(128, 128, 64, 8, PALETTE_LEGEND.h, 'opacity=".8"')}</g>`;
  return svgDocument({ width: 960, height: 640, viewBox: '0 0 960 640', title: 'grr terminal', description: 'A dimensional cream and orange terminal with a glowing screen and a tiny crab perched on its top edge.', body });
};

const artMail = () => svgDocument({
  width: 480,
  height: 360,
  viewBox: '0 0 480 360',
  title: 'Mail illustration',
  description: 'A dimensional envelope with a small blue paper plane.',
  body: `${contactShadow(240, 304, 152, 20)}<g shape-rendering="crispEdges">${polygon([[84, 140], [240, 92], [404, 140], [384, 284], [96, 284]], PALETTE_LEGEND.o)}${polygon([[100, 144], [240, 104], [388, 144], [372, 268], [112, 268]], PALETTE_LEGEND.d)}${polygon([[100, 140], [240, 100], [388, 140], [364, 160], [240, 124], [116, 160]], PALETTE_LEGEND.c)}${polygon([[112, 156], [240, 224], [368, 156], [376, 264], [104, 264]], PALETTE_LEGEND.c)}${polygon([[112, 156], [240, 224], [240, 248], [104, 180]], PALETTE_LEGEND.w)}${polygon([[368, 156], [240, 224], [240, 248], [376, 180]], PALETTE_LEGEND.h)}${line(112, 160, 240, 232, PALETTE_LEGEND.o, 8)}${line(368, 160, 240, 232, PALETTE_LEGEND.o, 8)}${line(104, 264, 240, 132, PALETTE_LEGEND.s, 8, 'opacity=".8"')}${line(376, 264, 240, 132, PALETTE_LEGEND.d, 8, 'opacity=".8"')}${polygon([[282, 72], [356, 96], [306, 148], [250, 122]], PALETTE_LEGEND.o)}${polygon([[288, 80], [344, 98], [304, 138], [262, 120]], PALETTE_LEGEND.u)}${polygon([[288, 80], [304, 138], [262, 120]], PALETTE_LEGEND.h)}${polygon([[304, 138], [344, 98], [312, 120]], '#1A120B', 'opacity=".8"')}${rect(136, 244, 64, 12, PALETTE_LEGEND.p, 'opacity=".55"')}</g>`
});

const artCalendar = () => svgDocument({
  width: 480,
  height: 360,
  viewBox: '0 0 480 360',
  title: 'Calendar illustration',
  description: 'A dimensional torn calendar page with a small clock.',
  body: `${contactShadow(240, 314, 148, 18)}<g shape-rendering="crispEdges">${polygon([[96, 76], [360, 76], [360, 292], [336, 280], [320, 300], [296, 284], [272, 304], [248, 284], [224, 300], [200, 284], [176, 304], [152, 284], [128, 300], [96, 284]], PALETTE_LEGEND.o)}${polygon([[112, 88], [348, 88], [348, 276], [328, 268], [312, 288], [288, 272], [264, 292], [240, 272], [216, 292], [192, 272], [168, 292], [144, 272], [112, 284]], PALETTE_LEGEND.c)}${polygon([[96, 76], [360, 76], [360, 120], [96, 120]], PALETTE_LEGEND.m)}${polygon([[112, 88], [344, 88], [344, 104], [112, 104]], PALETTE_LEGEND.p)}${rect(128, 56, 24, 44, PALETTE_LEGEND.o)}${rect(136, 60, 8, 32, PALETTE_LEGEND.w)}${rect(304, 56, 24, 44, PALETTE_LEGEND.o)}${rect(312, 60, 8, 32, PALETTE_LEGEND.w)}${rect(144, 148, 40, 32, PALETTE_LEGEND.h)}${rect(208, 148, 40, 32, PALETTE_LEGEND.w)}${rect(272, 148, 40, 32, PALETTE_LEGEND.h)}${rect(144, 204, 40, 32, PALETTE_LEGEND.w)}${rect(208, 204, 40, 32, PALETTE_LEGEND.h)}${polygon([[308, 214], [344, 188], [384, 212], [404, 252], [384, 296], [340, 308], [300, 288], [284, 248]], PALETTE_LEGEND.o)}${polygon([[316, 220], [344, 200], [376, 220], [392, 252], [376, 284], [340, 296], [308, 280], [296, 248]], PALETTE_LEGEND.b)}${polygon([[324, 228], [344, 214], [368, 228], [380, 252], [368, 276], [340, 284], [316, 272], [308, 248]], PALETTE_LEGEND.c)}${line(344, 224, 344, 252, PALETTE_LEGEND.o, 8)}${line(344, 252, 368, 264, PALETTE_LEGEND.o, 8)}${rect(340, 248, 8, 8, PALETTE_LEGEND.d)}</g>`
});

const artDrive = () => svgDocument({
  width: 480,
  height: 360,
  viewBox: '0 0 480 360',
  title: 'Drive illustration',
  description: 'A dimensional orange folder with a stacked file edge.',
  body: `${contactShadow(240, 306, 156, 20)}<g shape-rendering="crispEdges">${polygon([[140, 76], [344, 76], [376, 116], [376, 276], [112, 276], [112, 100]], PALETTE_LEGEND.o)}${polygon([[152, 88], [332, 88], [356, 124], [356, 260], [128, 260], [128, 112]], PALETTE_LEGEND.c)}${polygon([[152, 88], [332, 88], [356, 124], [336, 132], [128, 132], [128, 112]], PALETTE_LEGEND.h)}${rect(196, 112, 96, 12, PALETTE_LEGEND.w)}${rect(180, 140, 128, 12, PALETTE_LEGEND.w)}${polygon([[76, 132], [172, 132], [204, 104], [400, 104], [416, 276], [88, 276]], PALETTE_LEGEND.o)}${polygon([[88, 140], [164, 140], [204, 116], [388, 116], [400, 260], [100, 260]], PALETTE_LEGEND.d)}${polygon([[88, 140], [164, 140], [204, 116], [388, 116], [400, 164], [88, 164]], PALETTE_LEGEND.m)}${polygon([[88, 164], [400, 164], [400, 260], [100, 260]], PALETTE_LEGEND.b)}${polygon([[104, 176], [384, 176], [384, 188], [104, 188]], PALETTE_LEGEND.p)}${polygon([[100, 260], [400, 260], [400, 276], [88, 276]], PALETTE_LEGEND.s)}</g>`
});

const artContacts = () => svgDocument({
  width: 480,
  height: 360,
  viewBox: '0 0 480 360',
  title: 'Contacts illustration',
  description: 'A dimensional contact card with a friendly generic avatar.',
  body: `${contactShadow(240, 310, 144, 18)}<g shape-rendering="crispEdges">${polygon([[96, 76], [384, 76], [408, 100], [408, 292], [96, 292]], PALETTE_LEGEND.o)}${polygon([[112, 92], [384, 92], [392, 100], [392, 276], [112, 276]], PALETTE_LEGEND.c)}${polygon([[112, 92], [384, 92], [384, 108], [112, 108]], PALETTE_LEGEND.h)}${polygon([[336, 108], [384, 108], [384, 276], [336, 276]], PALETTE_LEGEND.w, 'opacity=".45"')}${polygon([[140, 144], [180, 120], [220, 144], [220, 188], [180, 212], [140, 188]], PALETTE_LEGEND.o)}${polygon([[148, 148], [180, 128], [212, 148], [212, 180], [180, 200], [148, 180]], PALETTE_LEGEND.u)}${rect(160, 152, 40, 28, PALETTE_LEGEND.h)}${rect(168, 156, 12, 12, PALETTE_LEGEND.w)}${polygon([[144, 228], [156, 204], [180, 192], [204, 204], [220, 228], [220, 248], [144, 248]], PALETTE_LEGEND.o)}${polygon([[152, 232], [164, 212], [180, 204], [200, 212], [212, 232], [212, 240], [152, 240]], PALETTE_LEGEND.p)}${rect(260, 148, 84, 12, PALETTE_LEGEND.d)}${rect(260, 176, 56, 12, PALETTE_LEGEND.p)}${rect(260, 216, 104, 12, PALETTE_LEGEND.m)}${rect(260, 244, 72, 12, PALETTE_LEGEND.s, 'opacity=".7"')}${rect(120, 116, 12, 12, PALETTE_LEGEND.b)}</g>`
});

const artChat = () => svgDocument({
  width: 480,
  height: 360,
  viewBox: '0 0 480 360',
  title: 'Chat illustration',
  description: 'A dimensional pair of speech bubbles with a single reaction dot.',
  body: `${contactShadow(240, 304, 152, 20)}<g shape-rendering="crispEdges">${polygon([[120, 92], [340, 92], [364, 116], [364, 220], [228, 220], [188, 264], [192, 220], [120, 220]], PALETTE_LEGEND.o)}${polygon([[132, 104], [336, 104], [352, 120], [352, 204], [224, 204], [204, 232], [208, 204], [132, 204]], PALETTE_LEGEND.d)}${polygon([[132, 104], [336, 104], [352, 120], [336, 132], [132, 132]], PALETTE_LEGEND.m)}${polygon([[168, 144], [244, 144], [264, 164], [264, 212], [168, 212], [148, 232], [152, 212], [148, 164]], PALETTE_LEGEND.o)}${polygon([[180, 156], [240, 156], [252, 168], [252, 200], [180, 200], [164, 216], [168, 200], [164, 168]], PALETTE_LEGEND.c)}${polygon([[292, 236], [364, 236], [384, 256], [384, 292], [312, 292], [300, 316], [300, 292], [292, 292]], PALETTE_LEGEND.o)}${polygon([[304, 248], [364, 248], [372, 260], [372, 280], [304, 280], [304, 292], [292, 280], [292, 260]], PALETTE_LEGEND.g)}${rect(320, 260, 16, 16, PALETTE_LEGEND.h)}${rect(196, 168, 24, 16, PALETTE_LEGEND.b)}${rect(240, 184, 16, 16, PALETTE_LEGEND.d)}</g>`
});

const artForms = () => svgDocument({
  width: 480,
  height: 360,
  viewBox: '0 0 480 360',
  title: 'Forms illustration',
  description: 'A dimensional form sheet with checked boxes and a pencil.',
  body: `${contactShadow(240, 310, 144, 18)}<g shape-rendering="crispEdges">${polygon([[92, 56], [340, 56], [364, 80], [364, 300], [92, 300]], PALETTE_LEGEND.o)}${polygon([[108, 72], [332, 72], [348, 88], [348, 284], [108, 284]], PALETTE_LEGEND.c)}${polygon([[108, 72], [332, 72], [332, 88], [108, 88]], PALETTE_LEGEND.h)}${rect(136, 120, 32, 32, PALETTE_LEGEND.o)}${rect(144, 128, 16, 16, PALETTE_LEGEND.g)}${line(148, 136, 156, 144, PALETTE_LEGEND.h, 4)}${line(156, 144, 168, 124, PALETTE_LEGEND.h, 4)}${rect(200, 128, 100, 12, PALETTE_LEGEND.d)}${rect(200, 148, 76, 12, PALETTE_LEGEND.p)}${rect(136, 180, 32, 32, PALETTE_LEGEND.o)}${rect(144, 188, 16, 16, PALETTE_LEGEND.g)}${line(148, 196, 156, 204, PALETTE_LEGEND.h, 4)}${line(156, 204, 168, 184, PALETTE_LEGEND.h, 4)}${rect(200, 188, 124, 12, PALETTE_LEGEND.m)}${rect(200, 208, 92, 12, PALETTE_LEGEND.b)}${polygon([[284, 276], [304, 236], [356, 260], [336, 300]], PALETTE_LEGEND.o)}${polygon([[296, 272], [308, 244], [344, 260], [332, 288]], PALETTE_LEGEND.b)}${polygon([[308, 244], [356, 260], [348, 272], [300, 256]], PALETTE_LEGEND.w)}${polygon([[284, 276], [304, 236], [312, 240], [292, 280]], PALETTE_LEGEND.p)}${polygon([[284, 276], [292, 280], [336, 300], [332, 308]], PALETTE_LEGEND.s)}</g>`
});

const glyphBase = (title, body) => svgDocument({
  width: 48,
  height: 48,
  viewBox: '0 0 48 48',
  title,
  description: `${title} pixel glyph`,
  body: `<g shape-rendering="crispEdges">${body}</g>`
});

const glyphMail = () => glyphBase('Mail glyph', `${rect(6, 12, 36, 26, PALETTE_LEGEND.o)}${rect(8, 14, 32, 22, PALETTE_LEGEND.c)}${polygon([[10, 16], [38, 16], [24, 30]], PALETTE_LEGEND.h)}${line(10, 16, 24, 30, PALETTE_LEGEND.o, 2)}${line(38, 16, 24, 30, PALETTE_LEGEND.o, 2)}${line(10, 34, 22, 22, PALETTE_LEGEND.s, 2, 'opacity=".6"')}${line(38, 34, 26, 22, PALETTE_LEGEND.d, 2, 'opacity=".6"')}${rect(10, 8, 10, 6, PALETTE_LEGEND.p)}${rect(32, 34, 8, 6, PALETTE_LEGEND.d)}`);

const glyphCalendar = () => glyphBase('Calendar glyph', `${rect(8, 10, 32, 30, PALETTE_LEGEND.o)}${rect(10, 12, 28, 26, PALETTE_LEGEND.c)}${rect(10, 12, 28, 8, PALETTE_LEGEND.m)}${rect(14, 6, 4, 10, PALETTE_LEGEND.o)}${rect(30, 6, 4, 10, PALETTE_LEGEND.o)}${rect(14, 24, 6, 6, PALETTE_LEGEND.p)}${rect(22, 24, 6, 6, PALETTE_LEGEND.w)}${rect(30, 24, 6, 6, PALETTE_LEGEND.p)}${rect(14, 32, 6, 6, PALETTE_LEGEND.w)}${rect(22, 32, 6, 6, PALETTE_LEGEND.p)}${rect(30, 32, 6, 6, PALETTE_LEGEND.w)}`);

const glyphDrive = () => glyphBase('Drive glyph', `${rect(10, 8, 28, 30, PALETTE_LEGEND.o)}${rect(12, 10, 24, 26, PALETTE_LEGEND.c)}${rect(18, 14, 12, 4, PALETTE_LEGEND.h)}${path('M6 18H18L22 14H42V38H6Z', PALETTE_LEGEND.o)}${path('M8 20H18L22 16H40V36H8Z', PALETTE_LEGEND.b)}${rect(10, 24, 28, 4, PALETTE_LEGEND.p)}${rect(8, 36, 34, 4, PALETTE_LEGEND.s)}`);

const glyphContacts = () => glyphBase('Contacts glyph', `${rect(6, 10, 36, 30, PALETTE_LEGEND.o)}${rect(8, 12, 32, 26, PALETTE_LEGEND.c)}${path('M18 22a6 6 0 1 0 12 0a6 6 0 1 0-12 0', PALETTE_LEGEND.u)}${path('M12 36c2-8 8-10 12-10s10 2 12 10', PALETTE_LEGEND.b)}${rect(32, 18, 6, 4, PALETTE_LEGEND.h)}${rect(32, 24, 6, 4, PALETTE_LEGEND.h)}${rect(10, 8, 6, 6, PALETTE_LEGEND.p)}`);

const glyphChat = () => glyphBase('Chat glyph', `${path('M8 10H36L42 16V30H24L18 38V30H8Z', PALETTE_LEGEND.o)}${path('M10 12H34L40 18V28H22L18 32V28H10Z', PALETTE_LEGEND.b)}${path('M18 18H34V30H26L22 34V30H18Z', PALETTE_LEGEND.o)}${path('M20 20H32V28H25L23 31V28H20Z', PALETTE_LEGEND.c)}${rect(30, 34, 8, 8, PALETTE_LEGEND.g)}${rect(32, 36, 4, 4, PALETTE_LEGEND.h)}`);

const glyphForms = () => glyphBase('Forms glyph', `${path('M8 6H32L40 14V42H8Z', PALETTE_LEGEND.o)}${path('M10 8H30L38 16V40H10Z', PALETTE_LEGEND.c)}${rect(14, 16, 8, 8, PALETTE_LEGEND.o)}${rect(16, 18, 4, 4, PALETTE_LEGEND.g)}${rect(26, 18, 8, 4, PALETTE_LEGEND.d)}${rect(14, 28, 8, 8, PALETTE_LEGEND.o)}${rect(16, 30, 4, 4, PALETTE_LEGEND.g)}${rect(26, 30, 8, 4, PALETTE_LEGEND.d)}${polygon([[30, 38], [36, 20], [40, 22], [34, 40]], PALETTE_LEGEND.o)}${polygon([[32, 37], [37, 22], [39, 23], [34, 38]], PALETTE_LEGEND.b)}`);

const makeHeadMap = () => {
  const map = Array.from({ length: 32 }, () => Array(32).fill('.'));
  const set = (x, y, color) => { if (x >= 0 && y >= 0 && x < 32 && y < 32) map[y][x] = color; };
  const ellipse = (cx, cy, rx, ry, color) => { for (let y = 0; y < 32; y += 1) for (let x = 0; x < 32; x += 1) if (((x + 0.5 - cx) / rx) ** 2 + ((y + 0.5 - cy) / ry) ** 2 <= 1) set(x, y, color); };
  ellipse(16, 19, 14, 10, 'o');
  ellipse(16, 19, 12.5, 8.5, 'm');
  ellipse(14, 17, 10, 6, 'b');
  ellipse(11, 14, 5, 3, 'p');
  ellipse(8, 12, 2, 1, 'h');
  for (const [x, y, color] of [[3, 15, 'o'], [2, 18, 'o'], [4, 21, 'o'], [28, 15, 'o'], [29, 18, 'o'], [27, 21, 'o'], [5, 16, 'b'], [26, 16, 'p'], [5, 20, 'd'], [26, 20, 'd']]) set(x, y, color);
  for (const [x, y] of [[10, 10], [11, 10], [10, 11], [20, 10], [21, 10], [20, 11]]) { set(x, y, 'o'); set(x + 1, y, 'h'); set(x, y + 1, 'h'); }
  for (const [x, y] of [[11, 12], [12, 12], [11, 13], [21, 12], [22, 12], [21, 13]]) set(x, y, 'k');
  set(12, 12, 'h'); set(22, 12, 'h');
  for (const [x, y] of [[13, 21], [14, 22], [15, 22], [16, 22], [17, 22], [18, 22], [19, 21]]) set(x, y, 'k');
  set(8, 20, 'r'); set(23, 20, 'r');
  set(14, 25, 'd'); set(18, 25, 'd');
  return map;
};

const HEAD_MAP = makeHeadMap();

const headLayer = (x, y, scale) => `<g shape-rendering="crispEdges">${mapRuns(HEAD_MAP).map((run) => `<path fill="${PALETTE_LEGEND[run.color]}" d="M${run.x * scale + x} ${run.y * scale + y}h${run.width * scale}v${run.height * scale}h-${run.width * scale}z"/>`).join('')}</g>`;

const FONT = Object.freeze({
  ' ': ['00000', '00000', '00000', '00000', '00000', '00000', '00000'],
  G: ['01110', '10001', '10000', '10111', '10001', '10001', '01111'],
  g: ['00000', '01111', '10001', '10001', '01111', '00001', '01110'],
  o: ['00000', '01110', '10001', '10001', '10001', '10001', '01110'],
  l: ['00100', '00100', '00100', '00100', '00100', '00100', '01110'],
  e: ['00000', '01110', '10001', '11111', '10000', '10000', '01111'],
  t: ['01000', '01000', '11110', '01000', '01000', '01001', '00110'],
  s: ['00000', '01111', '10000', '01110', '00001', '00001', '11110'],
  f: ['00110', '01001', '01000', '11110', '01000', '01000', '01000'],
  r: ['00000', '10110', '11001', '10000', '10000', '10000', '10000'],
  m: ['00000', '11010', '10101', '10101', '10101', '10101', '10101'],
  h: ['10000', '10000', '10110', '11001', '10001', '10001', '10001'],
  i: ['00100', '00000', '01100', '00100', '00100', '00100', '01110'],
  n: ['00000', '10110', '11001', '10001', '10001', '10001', '10001'],
  a: ['00000', '01110', '00001', '01111', '10001', '10001', '01111'],
  '-': ['00000', '00000', '00000', '11111', '00000', '00000', '00000'],
  '/': ['00001', '00010', '00010', '00100', '01000', '01000', '10000'],
  '.': ['00000', '00000', '00000', '00000', '00000', '00110', '00110'],
  ':': ['00000', '00110', '00110', '00000', '00110', '00110', '00000']
});

const pixelText = (text, x, y, scale, fill, extra = '') => {
  const paths = [];
  [...text].forEach((character, index) => {
    const glyph = FONT[character] ?? FONT[' '];
    glyph.forEach((row, rowIndex) => {
      [...row].forEach((cell, columnIndex) => {
        if (cell !== '1') return;
        const px = x + (index * 6 + columnIndex) * scale;
        const py = y + rowIndex * scale;
        paths.push(`M${px} ${py}h${scale}v${scale}h-${scale}z`);
      });
    });
  });
  return paths.length ? `<path fill="${fill}" d="${paths.join('')}"${extra ? ` ${extra}` : ''}/>` : '';
};

const faviconSvg = (width) => svgDocument({
  width,
  height: width,
  viewBox: '0 0 32 32',
  title: 'grr',
  description: 'A compact orange crab head with dimensional pixel shading.',
  body: headLayer(0, 0, 1)
});

const logoSvg = () => svgDocument({
  width: 256,
  height: 256,
  viewBox: '0 0 256 256',
  title: 'Google Rust Rewrite',
  description: 'The dimensional orange grr crab mascot.',
  body: headLayer(0, 0, 8)
});

const logoWordmarkSvg = () => svgDocument({
  width: 640,
  height: 256,
  viewBox: '0 0 640 256',
  title: 'Google Rust Rewrite',
  description: 'The grr crab mascot with its pixel wordmark.',
  body: `${headLayer(0, 0, 8)}${pixelText('grr', 288, 72, 16, PALETTE_LEGEND.o)}${pixelText('grr', 280, 64, 16, PALETTE_LEGEND.p)}${rect(296, 80, 16, 8, PALETTE_LEGEND.h)}`
});

const ogSvg = () => {
  const body = `${rect(0, 0, 1200, 630, PALETTE_LEGEND.c)}${polygon([[0, 0], [1200, 0], [1200, 176], [0, 328]], PALETTE_LEGEND.h, 'opacity=".55"')}${polygon([[0, 502], [1200, 348], [1200, 630], [0, 630]], PALETTE_LEGEND.w, 'opacity=".6"')}${contactShadow(236, 496, 170, 22)}${terminalCrab(ASCII_MAPS.idle, 40, 168, 5)}${pixelText('grr', 560, 112, 24, PALETTE_LEGEND.s, 'opacity=".22"')}${pixelText('grr', 552, 104, 24, PALETTE_LEGEND.m)}${pixelText('grr', 544, 96, 24, PALETTE_LEGEND.p)}${rect(568, 112, 32, 16, PALETTE_LEGEND.h, 'opacity=".75"')}${pixelText('Google tools from the terminal', 480, 304, 4, PALETTE_LEGEND.k)}${polygon([[632, 424], [1096, 384], [1136, 448], [672, 488]], PALETTE_LEGEND.o)}${polygon([[648, 428], [1088, 392], [1116, 440], [676, 476]], PALETTE_LEGEND.d)}${polygon([[664, 432], [1080, 400], [1096, 428], [680, 460]], PALETTE_LEGEND.m)}${polygon([[680, 440], [1072, 412], [1080, 430], [688, 458]], PALETTE_LEGEND.p)}${rect(728, 430, 24, 16, PALETTE_LEGEND.h)}${rect(768, 426, 80, 12, PALETTE_LEGEND.w)}${rect(864, 422, 128, 12, PALETTE_LEGEND.b)}${rect(768, 450, 168, 12, PALETTE_LEGEND.s)}$`;
  return svgDocument({ width: 1200, height: 630, viewBox: '0 0 1200 630', title: 'grr — Google tools from the terminal', description: 'A warm paper product card with the dimensional grr crab, wordmark, tagline, and a small terminal slab.', body });
};

const ASCII_SOURCES = Object.freeze({
  idle: buildIdleMap().map((row) => row.join('')).join('\n'),
  wave: buildWaveMap().map((row) => row.join('')).join('\n'),
  sleep: buildSleepMap().map((row) => row.join('')).join('\n'),
  celebrate: buildCelebrateMap().map((row) => row.join('')).join('\n')
});

const ASCII_MAPS = Object.freeze(Object.fromEntries(
  Object.entries(ASCII_SOURCES).map(([name, source]) => [name, parseAscii(name, source)])
));

const files = new Map([
  [resolve(assetDirectory, 'mascot-idle.svg'), mascotSvg(ASCII_MAPS.idle, 'idle', 'A friendly orange crab resting with chunky raised claws, glossy eyes, a small smile, and a separate soft contact shadow.')],
  [resolve(assetDirectory, 'mascot-wave.svg'), mascotSvg(ASCII_MAPS.wave, 'waving', 'A friendly orange crab lifting one tiny claw in a wave, shaded with a warm top light.')],
  [resolve(assetDirectory, 'mascot-sleep.svg'), mascotSvg(ASCII_MAPS.sleep, 'sleeping', 'A friendly orange crab lying down with closed eyes and tiny pixel sleep marks.')],
  [resolve(assetDirectory, 'mascot-celebrate.svg'), mascotSvg(ASCII_MAPS.celebrate, 'celebrating', 'A joyful orange crab with both claws raised, a wide smile, and small warm confetti pixels.')],
  [resolve(assetDirectory, 'hero-terminal.svg'), heroTerminal()],
  [resolve(assetDirectory, 'art-mail.svg'), artMail()],
  [resolve(assetDirectory, 'art-calendar.svg'), artCalendar()],
  [resolve(assetDirectory, 'art-drive.svg'), artDrive()],
  [resolve(assetDirectory, 'art-contacts.svg'), artContacts()],
  [resolve(assetDirectory, 'art-chat.svg'), artChat()],
  [resolve(assetDirectory, 'art-forms.svg'), artForms()],
  [resolve(assetDirectory, 'glyph-mail.svg'), glyphMail()],
  [resolve(assetDirectory, 'glyph-calendar.svg'), glyphCalendar()],
  [resolve(assetDirectory, 'glyph-drive.svg'), glyphDrive()],
  [resolve(assetDirectory, 'glyph-contacts.svg'), glyphContacts()],
  [resolve(assetDirectory, 'glyph-chat.svg'), glyphChat()],
  [resolve(assetDirectory, 'glyph-forms.svg'), glyphForms()],
  [resolve(publicDirectory, 'favicon.svg'), faviconSvg(32)],
  [resolve(publicDirectory, 'favicon-16.svg'), faviconSvg(16)],
  [resolve(publicDirectory, 'favicon-32.svg'), faviconSvg(32)],
  [resolve(publicDirectory, 'logo.svg'), logoSvg()],
  [resolve(publicDirectory, 'logo-wordmark.svg'), logoWordmarkSvg()],
  [resolve(publicDirectory, 'og.svg'), ogSvg()]
]);

const rejected = [
  resolve(publicDirectory, 'assets', 'patterns', 'dither-tile.svg'),
  resolve(publicDirectory, 'assets', 'patterns', 'halftone-tile.svg'),
  resolve(assetDirectory, 'corner-ornament.svg'),
  resolve(assetDirectory, 'terminal-frame.svg')
];

mkdirSync(assetDirectory, { recursive: true });
mkdirSync(publicDirectory, { recursive: true });

for (const [path, content] of files) {
  writeFileSync(path, content, 'utf8');
}

for (const path of rejected) {
  if (existsSync(path)) rmSync(path);
}

console.log(`Generated ${files.size} SVG assets with 64x64 mascot pixel maps.`);
