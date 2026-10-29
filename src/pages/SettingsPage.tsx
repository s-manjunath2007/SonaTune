import React, { useState } from 'react';
import type { AppSettings, MediaToolInfo } from '../types';
import { apiResetSettings } from '../api';

interface SettingsPageProps {
  settings: AppSettings;
  onChangeSettings: (newSettings: AppSettings) => void;
  mediaTools: MediaToolInfo;
}

export const SettingsPage: React.FC<SettingsPageProps> = ({
  settings,
  onChangeSettings,
  mediaTools,
}) => {
  const [resetConfirmOpen, setResetConfirmOpen] = useState<boolean>(false);

  const handleReset = async () => {
    try {
      const defaults = await apiResetSettings();
      onChangeSettings(defaults);
      setResetConfirmOpen(false);
    } catch (_) {}
  };

  return (
    <div style={{ maxWidth: '800px', margin: '0 auto' }}>
      {/* Appearance */}
      <div className="panel">
        <div className="panel-header">Appearance & Theme</div>
        <div className="panel-body">
          <div className="control-group">
            <label className="control-label">Interface Color Theme</label>
            <div style={{ display: 'flex', gap: '12px', marginTop: '6px' }}>
              <label style={{ display: 'flex', alignItems: 'center', gap: '6px', cursor: 'pointer' }}>
                <input
                  type="radio"
                  name="theme"
                  checked={settings.theme === 'light'}
                  onChange={() => onChangeSettings({ ...settings, theme: 'light' })}
                />
                <span>Light Theme (Traditional Desktop Default)</span>
              </label>

              <label style={{ display: 'flex', alignItems: 'center', gap: '6px', cursor: 'pointer' }}>
                <input
                  type="radio"
                  name="theme"
                  checked={settings.theme === 'dark'}
                  onChange={() => onChangeSettings({ ...settings, theme: 'dark' })}
                />
                <span>Dark Theme (Neutral Charcoal)</span>
              </label>
            </div>
          </div>
        </div>
      </div>

      {/* Audio Engine Defaults */}
      <div className="panel">
        <div className="panel-header">Audio Engine & Processing Defaults</div>
        <div className="panel-body">
          <div className="control-group">
            <label style={{ display: 'flex', alignItems: 'center', gap: '8px', cursor: 'pointer' }}>
              <input
                type="checkbox"
                checked={settings.current_config.limiter_enabled}
                onChange={(e) =>
                  onChangeSettings({
                    ...settings,
                    current_config: { ...settings.current_config, limiter_enabled: e.target.checked },
                  })
                }
              />
              <span>Enable Soft-Knee Limiter by Default (Prevents digital clipping and overs)</span>
            </label>
          </div>

          <div className="control-group">
            <label style={{ display: 'flex', alignItems: 'center', gap: '8px', cursor: 'pointer' }}>
              <input
                type="checkbox"
                checked={settings.a_b_loudness_matched}
                onChange={(e) => onChangeSettings({ ...settings, a_b_loudness_matched: e.target.checked })}
              />
              <span>Enable Loudness-Matched A/B Comparison Preview by default</span>
            </label>
          </div>
        </div>
      </div>

      {/* System Tray & Startup */}
      <div className="panel">
        <div className="panel-header">Background & Startup Behavior</div>
        <div className="panel-body">
          <div className="control-group">
            <label style={{ display: 'flex', alignItems: 'center', gap: '8px', cursor: 'pointer' }}>
              <input
                type="checkbox"
                checked={settings.minimize_to_tray}
                onChange={(e) => onChangeSettings({ ...settings, minimize_to_tray: e.target.checked })}
              />
              <span>Minimize to System Tray on close</span>
            </label>
            <div style={{ fontSize: '11px', color: 'var(--text-muted)', marginLeft: '22px', marginTop: '2px' }}>
              Keeps background audio routing active while running unobtrusively in the Windows notification area.
            </div>
          </div>

          <div className="control-group" style={{ marginBottom: 0 }}>
            <label style={{ display: 'flex', alignItems: 'center', gap: '8px', cursor: 'pointer' }}>
              <input
                type="checkbox"
                checked={settings.start_with_windows}
                onChange={(e) => onChangeSettings({ ...settings, start_with_windows: e.target.checked })}
              />
              <span>Start SonaTune automatically on Windows logon (Disabled by default)</span>
            </label>
          </div>
        </div>
      </div>

      {/* Media Tools / FFmpeg Detection */}
      <div className="panel">
        <div className="panel-header">Media Processing Subsystem (FFmpeg & FFprobe)</div>
        <div className="panel-body">
          <div style={{ display: 'flex', flexDirection: 'column', gap: '6px', fontSize: '12.5px' }}>
            <div style={{ display: 'flex', justifyContent: 'space-between' }}>
              <span>FFmpeg Binary Status:</span>
              <span className={`status-badge ${mediaTools.ffmpeg_available ? 'active' : ''}`}>
                {mediaTools.ffmpeg_available ? 'Installed & Ready' : 'Not Found in PATH'}
              </span>
            </div>

            {mediaTools.ffmpeg_path && (
              <div style={{ fontSize: '11.5px', color: 'var(--text-secondary)' }}>
                Location: <code>{mediaTools.ffmpeg_path}</code>
              </div>
            )}

            {mediaTools.version && (
              <div style={{ fontSize: '11px', color: 'var(--text-muted)' }}>
                {mediaTools.version}
              </div>
            )}

            <div style={{ marginTop: '8px', fontSize: '11.5px', color: 'var(--text-muted)', lineHeight: '1.4' }}>
              Note: SonaTune does not require FFmpeg for pure audio files (WAV, MP3, FLAC) as it includes native Rust decoders and encoders. FFmpeg is utilized for demuxing and remuxing video audio streams without video recompression.
            </div>
          </div>
        </div>
      </div>

      {/* Factory Reset */}
      <div className="panel">
        <div className="panel-header">Reset & Maintenance</div>
        <div className="panel-body">
          <p style={{ fontSize: '12px', color: 'var(--text-secondary)', marginBottom: '10px' }}>
            Restore all SonaTune sound controls, presets, device preferences, and application settings to their original factory defaults.
          </p>

          {!resetConfirmOpen ? (
            <button className="btn btn-danger btn-sm" onClick={() => setResetConfirmOpen(true)}>
              Reset All Settings to Factory Defaults...
            </button>
          ) : (
            <div style={{ display: 'flex', gap: '8px', alignItems: 'center' }}>
              <span style={{ fontSize: '12px', color: 'var(--danger)', fontWeight: 600 }}>
                Are you sure you want to reset all settings?
              </span>
              <button className="btn btn-danger btn-sm" onClick={handleReset}>
                Yes, Reset Everything
              </button>
              <button className="btn btn-sm" onClick={() => setResetConfirmOpen(false)}>
                Cancel
              </button>
            </div>
          )}
        </div>
      </div>

      {/* About Box */}
      <div className="panel">
        <div className="panel-header">About SonaTune</div>
        <div className="panel-body">
          <div style={{ fontSize: '12.5px', lineHeight: '1.6' }}>
            <div style={{ fontWeight: 600, fontSize: '13px' }}>SonaTune v0.1.0 (Windows Desktop Release)</div>
            <div style={{ color: 'var(--text-secondary)', marginTop: '4px' }}>
              Native Windows Audio Enhancement Utility • Built with Tauri 2, Rust DSP, and React
            </div>
            <div style={{ marginTop: '10px', color: 'var(--text-muted)', fontSize: '11.5px' }}>
              Principles: 100% offline signal processing. No remote accounts, no cloud servers, no telemetry, and zero AI API keys required for audio enhancement.
            </div>
          </div>
        </div>
      </div>
    </div>
  );
};
