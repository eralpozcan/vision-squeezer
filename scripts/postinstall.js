#!/usr/bin/env node

// postinstall: build the Rust binary if cargo is available, otherwise
// check for pre-built binaries.

const { execSync } = require("child_process");
const fs = require("fs");
const path = require("path");

const binDir = path.join(__dirname, "..", "bin");
const binaryName = "vision-squeezer-mcp";
const targetBinary = path.join(binDir, binaryName);

// If binary already exists (pre-built release), skip
if (fs.existsSync(targetBinary)) {
  console.log("✅ vision-squeezer-mcp binary found.");
  process.exit(0);
}

// Try building with cargo
try {
  execSync("cargo --version", { stdio: "ignore" });
} catch {
  console.error("❌ Rust toolchain not found. Install from https://rustup.rs");
  console.error("   Then run: cargo build --release && cp target/release/vision-squeezer-mcp bin/");
  process.exit(1);
}

console.log("🔨 Building vision-squeezer-mcp from source...");
try {
  execSync("cargo build --release", {
    cwd: path.join(__dirname, ".."),
    stdio: "inherit",
  });
  
  fs.mkdirSync(binDir, { recursive: true });
  const src = path.join(__dirname, "..", "target", "release", binaryName);
  fs.copyFileSync(src, targetBinary);
  fs.chmodSync(targetBinary, 0o755);
  console.log("✅ Built and installed vision-squeezer-mcp");
} catch (err) {
  console.error("❌ Build failed:", err.message);
  process.exit(1);
}
