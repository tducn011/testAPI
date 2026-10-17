# Expiry Control Center (`expiry-ctl`)

A high-performance, zero-dependency native Rust tool for:
1. **Server & Network Auto-Discovery**: Detects Tailscale IP, local LAN interfaces, and host identity.
2. **Docker Infrastructure Inspector**: Reads container health, uptime, and port mappings directly.
3. **Cloudflare Tunnel Sniffer**: Automatically resolves the live `*.trycloudflare.com` URL.
4. **Interactive Postman-like API Studio**: Terminal TUI with automatic Cookie Jar & CSRF synchronization, request runner, and E2E behavioral contract test suite.

---

## ⚡ Quick Download (Pre-built Binaries)

Go to the [**GitHub Releases**](https://github.com/tducn011/testAPI/releases) page to download:
* **Windows**: [`expiry-ctl.exe`](https://github.com/tducn011/testAPI/releases/latest/download/expiry-ctl.exe)
* **Linux**: [`expiry-ctl-linux`](https://github.com/tducn011/testAPI/releases/latest/download/expiry-ctl-linux)

---

## 🚀 How to Use on Windows

1. Download `expiry-ctl.exe` and place it in any folder.
2. Open PowerShell or Command Prompt in that folder.
3. Run the interactive Terminal UI:
   ```powershell
   .\expiry-ctl.exe
   ```
   * Use **↑ / ↓** arrows to navigate API endpoints (`Register`, `Login`, `Session`, `Food Entries`, `OCR Scan`).
   * Press **Tab** to switch target environments:
     * `[1]` Cloudflare Tunnel (HTTPS)
     * `[2]` Direct Localhost:3001
     * `[3]` Vercel Production
   * Press **Space** or **Enter** to dispatch requests.
   * Press **Q** or **Esc** to exit.

4. Quick CLI commands:
   ```powershell
   # Print real-time server IP, Docker, and Tunnel status
   .\expiry-ctl.exe status

   # Run automated 10-stage API verification suite
   .\expiry-ctl.exe test

   # Point to custom Tailscale host or domain
   .\expiry-ctl.exe --target http://100.80.6.17:3001 status
   ```

---

## 🐧 How to Use on Linux

```bash
chmod +x expiry-ctl-linux
./expiry-ctl-linux status
./expiry-ctl-linux
```

---

## 🛠️ Building from Source

Prerequisites: Rust toolchain (`rustc` & `cargo`).

### Native Linux Build
```bash
cargo build --release
# Output: target/release/expiry-ctl
```

### Windows `.exe` Cross-Compilation (from Linux)
```bash
rustup target add x86_64-pc-windows-gnu
sudo apt-get install -y gcc-mingw-w64-x86-64
cargo build --release --target x86_64-pc-windows-gnu
# Output: target/x86_64-pc-windows-gnu/release/expiry-ctl.exe
```

---

## 🛡️ Key Architectural Advantages

* **Pure `rustls` Engine**: Zero OpenSSL dependencies, guaranteed no `missing DLL` errors on fresh Windows installations.
* **Auto Session & Cookie Management**: Preserves `__Host-expiry_session` and `X-CSRF-Token` in memory across requests without manual copy-pasting.
* **CORS-Immune**: Operates directly on the OS network layer, bypassing web browser cross-origin restrictions.
