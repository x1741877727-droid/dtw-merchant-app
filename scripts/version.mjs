#!/usr/bin/env node
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const versionPath = path.join(root, 'VERSION');
const semver = /^\d+\.\d+\.\d+$/;

function version() {
  const value = fs.readFileSync(versionPath, 'utf8').trim();
  if (!semver.test(value)) throw new Error(`VERSION 不是 SemVer: ${value}`);
  return value;
}

function replaceFirstVersions(relativePath, value, count) {
  const target = path.join(root, relativePath);
  let seen = 0;
  const before = fs.readFileSync(target, 'utf8');
  const after = before.replace(/("version"\s*:\s*")[^"]+("\s*[,}])/g, (match, start, end) => {
    if (seen++ >= count) return match;
    return `${start}${value}${end}`;
  });
  if (seen < count) throw new Error(`${relativePath}: 找不到 ${count} 个 version 字段`);
  fs.writeFileSync(target, after);
}

function sync() {
  const value = version();
  replaceFirstVersions('package.json', value, 1);
  replaceFirstVersions('package-lock.json', value, 2);
  for (const file of ['src-tauri/Cargo.toml', 'src-tauri/Cargo.lock']) {
    const target = path.join(root, file);
    const before = fs.readFileSync(target, 'utf8');
    const pattern = /(name = "dtw-merchant"\s*\nversion = ")[^"]+(")/;
    if (!pattern.test(before)) throw new Error(`${file}: 找不到 dtw-merchant 版本`);
    const after = before.replace(pattern, `$1${value}$2`);
    fs.writeFileSync(target, after);
  }
  const tauriPath = path.join(root, 'src-tauri/tauri.conf.json');
  fs.writeFileSync(tauriPath, fs.readFileSync(tauriPath, 'utf8').replace(/("version"\s*:\s*")[^"]+(")/, `$1${value}$2`));
  for (const file of ['dist/index.html', 'dist/titlebar.html']) {
    const target = path.join(root, file);
    const before = fs.readFileSync(target, 'utf8');
    const marker = 'data-dtw-version>';
    const start = before.indexOf(marker);
    if (start < 0) throw new Error(`${file}: 找不到 data-dtw-version`);
    const valueStart = start + marker.length;
    const valueEnd = before.indexOf('<', valueStart);
    if (valueEnd < 0) throw new Error(`${file}: data-dtw-version 格式错误`);
    const after = before.slice(0, valueStart) + value + before.slice(valueEnd);
    fs.writeFileSync(target, after);
  }
}

function check() {
  const files = ['package.json', 'package-lock.json', 'src-tauri/Cargo.toml', 'src-tauri/Cargo.lock', 'src-tauri/tauri.conf.json', 'dist/index.html', 'dist/titlebar.html'];
  const before = new Map(files.map((file) => [file, fs.readFileSync(path.join(root, file), 'utf8')]));
  sync();
  const changed = files.filter((file) => before.get(file) !== fs.readFileSync(path.join(root, file), 'utf8'));
  if (changed.length) {
    for (const [file, content] of before) fs.writeFileSync(path.join(root, file), content);
    throw new Error(`版本未同步: ${changed.join(', ')}；运行 node scripts/version.mjs sync`);
  }
  console.log(`desktop ${version()}`);
}

function bump(current, kind) {
  const [major, minor, patch] = current.split('.').map(Number);
  if (kind === 'major') return `${major + 1}.0.0`;
  if (kind === 'minor') return `${major}.${minor + 1}.0`;
  if (kind === 'patch') return `${major}.${minor}.${patch + 1}`;
  throw new Error('只能 bump major、minor 或 patch');
}

const [command = 'show', value] = process.argv.slice(2);
if (command === 'show') console.log(version());
else if (command === 'check') check();
else if (command === 'sync') { sync(); check(); }
else if (command === 'set' || command === 'bump') {
  const next = command === 'set' ? value : bump(version(), value);
  if (!semver.test(next || '')) throw new Error(`版本号不符合 SemVer: ${next}`);
  fs.writeFileSync(versionPath, `${next}\n`);
  sync();
  check();
} else throw new Error('用法: version.mjs show | check | sync | set X.Y.Z | bump patch|minor|major');
