# SonaTune — Desktop Audio Enhancement Utility

**SonaTune** is a native, lightweight Windows desktop audio enhancement application designed to improve how music, movies, dialogue, games, and media sound through headphones, earbuds, and speakers.

---

## 1. Core Features & Capabilities

- **Real Audio DSP Engine:**
  - **10-Band Graphic Equalizer** (31 Hz, 63 Hz, 125 Hz, 250 Hz, 500 Hz, 1 kHz, 2 kHz, 4 kHz, 8 kHz, 16 kHz) powered by second-order IIR biquad peaking filters (Robert Bristow-Johnson formulas).
  - **Bass Boost & Cut** (Second-order low-shelf filter at 120 Hz).
  - **Treble Sparkle & Cut** (Second-order high-shelf filter at 7.5 kHz).
  - **Vocal Clarity Shaper** (Presence peaking boost at 3.0 kHz with low-mid boominess attenuation).
  - **Soft-Clipping Limiter** (-0.5 dB threshold with cubic knee and hyperbolic tangent asymptote) strictly guaranteeing peak headroom ($< 1.0$) and preventing digital clipping or harsh wraps.
  - **Binaural Virtual Spatializer** (Head-shadow crossfeed delay line providing subtle stereo widening without phase cancellation).
  - **Real-Time Parameter Smoothing** on all gain and volume controls to eliminate zipper noise and pops.

- **Audio File Studio:**
  - Inspect audio file metadata (WAV, MP3, FLAC, AAC, M4A, OGG) with native file dialog browsing.
  - Lossless and lossy decoding using Symphonia pure-Rust decoders.
  - Real-time A/B comparison preview player.
  - **Loudness-Matched Comparison:** Normalizes output gain based on RMS energy so users compare true tonal balance and clarity rather than mere volume differences.
  - Export to 16-bit PCM WAV (lossless pure-Rust), FLAC, and high-quality MP3 (320 kbps) with progress and cancellation support.
  - Non-destructive processing: source files are strictly protected from overwriting.

- **Video Audio Enhancement:**
  - Inspect video containers and streams using FFprobe.
  - Multi-track audio stream discovery (language, title, codecs, channels).
  - **Lossless Stream-Copy Remuxing:** Replaces the selected audio track with the enhanced DSP audio while copying the original video stream directly (`-c:v copy`). Video resolution, color space, frame rate, and subtitles are preserved at 100% quality with zero re-encoding loss.

- **Device-Aware Profiles & Output Management:**
  - Real-time device discovery via Windows WASAPI and CPAL.
  - Automatically loads device-specific profiles when switching between Headphones, Laptop Speakers, or External Monitors.
  - Displays true Windows device names, default status, sample rates, and channel layouts without synthetic device names.

- **Windows System-Wide Live Audio Routing:**
  - Complete actor-thread architecture: input capture stream and output render stream run on dedicated background threads with zero COM thread-affinity leaks and **zero unsafe code**.
  - Built-in Feedback Protection: Blocks accidental loopback to the same device to prevent acoustic squeal and runaway digital loops.
  - Diagnostic status reports real loop latency, buffer stability, and virtual audio endpoint detection (e.g. VB-CABLE or Virtual Audio Cable).

- **Visual Design:**
  - Practical, traditional desktop utility aesthetic inspired by foobar2000, Audacity, and VLC.
  - Clean light theme default with soft neutral borders, high-contrast typography, and an optional neutral dark theme.
  - Resizable window, standard Windows menus, keyboard navigation shortcuts, and system tray minimization.

- **Privacy & Offline Integrity:**
  - 100% offline signal processing.
  - No remote accounts, no cloud databases, no telemetry, and zero AI API keys required.

---

## 2. Supported Formats & Codecs

| Category | Format / Extension | Decoder | Encoder / Export |
|---|---|---|---|
| Lossless Audio | **WAV** (`.wav`) | Symphonia (Native Rust) | Hound (16-bit PCM Native Lossless) |
| Lossless Audio | **FLAC** (`.flac`) | Symphonia (Native Rust) | FFmpeg (Lossless FLAC) |
| Compressed Audio | **MP3** (`.mp3`) | Symphonia (Native Rust) | FFmpeg (libmp3lame 320 kbps) |
| Compressed Audio | **AAC / M4A** (`.aac`, `.m4a`) | Symphonia (Native Rust) | FFmpeg (AAC 256 kbps) |
| Compressed Audio | **Ogg Vorbis** (`.ogg`) | Symphonia (Native Rust) | FFmpeg (libvorbis) |
| Video Containers | **MP4, MKV, MOV, WebM, AVI** | FFprobe Stream Reader | FFmpeg (`-c:v copy` Stream Copy) |

---

## 3. Windows System-Wide Live Audio Setup Workflow

To process and enhance audio from ordinary Windows applications (such as web browsers, Spotify, media players, and games), follow this standard Windows audio routing procedure:

1. **Install a Virtual Audio Device:**
   - Download and install **VB-CABLE Driver** (from VB-Audio Software, freeware/donationware for 64-bit Windows 10/11).
   - Once installed, Windows will display `CABLE Input (VB-Audio Virtual Cable)` in your audio output list.
2. **Configure Windows Application Playback:**
   - Click the Windows Taskbar Speaker icon (or open Windows Settings > Sound).
   - Select `CABLE Input (VB-Audio Virtual Cable)` as the Windows Default Output Device.
3. **Select Physical Hardware in SonaTune:**
   - In SonaTune, navigate to **Devices & Routing** (`Ctrl + 4`).
   - Select your physical hardware (e.g., `Speakers (Realtek(R) Audio)` or your USB Headphones) as the destination output device.
4. **Start Live Enhancement:**
   - Click **"Start Live Audio Routing"**.
   - SonaTune captures audio from the virtual cable, runs it through the real-time DSP biquad filter chain and limiter, and plays it out to your physical headphones or speakers.
5. **Restoring Default Settings:**
   - When you are finished, click **"Stop Live Audio Route"** and switch the Windows default playback device back to your physical hardware. SonaTune never modifies your system default settings without your explicit action.

---

## 4. Development & Build Verification

### Prerequisites
- Windows 10 or Windows 11 (64-bit)
- [Node.js](https://nodejs.org/) (v18 or higher)
- [Rust](https://rustup.rs/) (v1.75 or higher)
- [FFmpeg](https://ffmpeg.org/download.html) (or install via `winget install Gyan.FFmpeg.Essentials`)

### Running Verification Tests
```powershell
# 1. Typecheck and verify frontend
npx tsc --noEmit
npm run build

# 2. Run DSP standalone engine tests (14 unit/integration tests)
cargo test --manifest-path dsp-engine/Cargo.toml

# 3. Run Tauri backend unit and integration tests (13 tests)
cargo test --manifest-path src-tauri/Cargo.toml
```

### Packaging Production Installers
```powershell
npx tauri build
```
Generates:
- **NSIS Setup:** `src-tauri/target/release/bundle/nsis/SonaTune_0.1.0_x64-setup.exe`
- **WiX MSI Package:** `src-tauri/target/release/bundle/msi/SonaTune_0.1.0_x64_en-US.msi`

---

## 5. Keyboard Shortcuts

- `Ctrl + 1`: Sound (Main Audio Enhancement)
- `Ctrl + 2`: Audio Files Studio
- `Ctrl + 3`: Video Audio Enhancement
- `Ctrl + 4`: Devices & Routing
- `Ctrl + 5`: Presets Library
- `Ctrl + 6` / `Ctrl + ,`: Settings & Preferences
- `Spacebar`: Toggle DSP Enhancement bypass
- `Ctrl + O`: Open audio file

---

## 6. Security, Reliability & Privacy

- **100% Local Processing:** SonaTune operates strictly offline. No telemetry, no accounts, no cloud backend, and zero AI API keys required.
- **Thread Safety & COM Invariants:** Audio streams and COM handles are strictly encapsulated on dedicated actor worker threads. No `unsafe` trait implementations are used.
- **Data Protection:** Source audio and video files are treated as read-only and will never be overwritten.
