import { readFileSync, writeFileSync } from "fs";
import { resolve, dirname } from "path";
import { fileURLToPath } from "url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const root = resolve(__dirname, "..");

const version = process.argv[2];
if (!version) {
  console.error("用法: node scripts/version-bump.mjs <version>");
  console.error("示例: node scripts/version-bump.mjs 0.2.0");
  process.exit(1);
}

// Validate semver format
if (!/^\d+\.\d+\.\d+$/.test(version)) {
  console.error(`无效的版本号格式: ${version} (应为 x.y.z)`);
  process.exit(1);
}

// 1. package.json
const pkgPath = resolve(root, "package.json");
const pkg = JSON.parse(readFileSync(pkgPath, "utf-8"));
const oldVersion = pkg.version;
pkg.version = version;
writeFileSync(pkgPath, JSON.stringify(pkg, null, 2) + "\n");
console.log(`package.json: ${oldVersion} -> ${version}`);

// 2. src-tauri/Cargo.toml
const cargoPath = resolve(root, "src-tauri/Cargo.toml");
let cargo = readFileSync(cargoPath, "utf-8");
cargo = cargo.replace(/^version = ".*"$/m, `version = "${version}"`);
writeFileSync(cargoPath, cargo);
console.log(`Cargo.toml: -> ${version}`);

// 3. src-tauri/tauri.conf.json
const tauriConfPath = resolve(root, "src-tauri/tauri.conf.json");
const tauriConf = JSON.parse(readFileSync(tauriConfPath, "utf-8"));
tauriConf.version = version;
writeFileSync(tauriConfPath, JSON.stringify(tauriConf, null, 2) + "\n");
console.log(`tauri.conf.json: -> ${version}`);

console.log(`\n版本已同步更新为 ${version}`);
console.log(`接下来运行:`);
console.log(`  git add -A && git commit -m "chore: 发布 v${version}"`);
console.log(`  git tag v${version}`);
console.log(`  git push origin master --tags`);
