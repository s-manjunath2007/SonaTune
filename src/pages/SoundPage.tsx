import React from 'react';
import type { DspConfig, Preset, AudioDeviceInfo } from '../types';

interface SoundPageProps {
  config: DspConfig;
  onChangeConfig: (newConfig: DspConfig) => void;
  presets: Preset[];
  activePresetId: string;
  onSelectPreset: (presetId: string) => void;
  devices: AudioDeviceInfo[];
  selectedDeviceId?: string;
  onSelectDevice: (deviceId: string) => void;
  onResetDefaults: () => void;
}

const EQ_FREQUENCIES = ['31 Hz', '63 Hz', '125 Hz', '250 Hz', '500 Hz', '1 kHz', '2 kHz', '4 kHz', '8 kHz', '16 kHz'];

export const SoundPage: React.FC<SoundPageProps> = ({
  config,
  onChangeConfig,
  presets,
  activePresetId,
  onSelectPreset,
  devices,
  selectedDeviceId,
  onSelectDevice,
  onResetDefaults,
}) => {
  const handleMasterChange = (val: number) => {
    onChangeConfig({ ...config, master_volume: val });
  };

  const handleBassChange = (val: number) => {
    onChangeConfig({ ...config, bass_gain_db: val });
  };

  const handleTrebleChange = (val: number) => {
    onChangeConfig({ ...config, treble_gain_db: val });
  };

  const handleVocalChange = (val: number) => {
    onChangeConfig({ ...config, vocal_clarity_db: val });
  };

  const handleEqBandChange = (index: number, val: number) => {
    const updated = [...config.eq_bands_db];
    updated[index] = val;
    onChangeConfig({ ...config, eq_bands_db: updated });
  };

  const handleSpatialWidthChange = (val: number) => {
    onChangeConfig({ ...config, spatial_width: val });
  };

  const activePreset = presets.find((p) => p.id === activePresetId);

  return (
    <div style={{ maxWidth: '1000px', margin: '0 auto' }}>
      {/* Top Banner: Device & Preset Bar */}
      <div className="panel" style={{ marginBottom: '14px' }}>
        <div className="panel-body" style={{ padding: '10px 14px', display: 'flex', gap: '20px', alignItems: 'center', justifyContent: 'space-between', flexWrap: 'wrap' }}>
          {/* Preset Selector */}
          <div style={{ display: 'flex', alignItems: 'center', gap: '10px', minWidth: '320px' }}>
            <label style={{ fontWeight: 600, fontSize: '12.5px', color: 'var(--text-secondary)' }}>Audio Profile:</label>
            <select
              className="select-input"
              style={{ width: '220px' }}
              value={activePresetId}
              onChange={(e) => onSelectPreset(e.target.value)}
            >
              <optgroup label="Factory Presets">
                {presets.filter((p) => p.is_builtin).map((p) => (
                  <option key={p.id} value={p.id}>
                    {p.name}
                  </option>
                ))}
              </optgroup>
              {presets.some((p) => !p.is_builtin) && (
                <optgroup label="Custom Profiles">
                  {presets.filter((p) => !p.is_builtin).map((p) => (
                    <option key={p.id} value={p.id}>
                      {p.name}
                    </option>
                  ))}
                </optgroup>
              )}
            </select>
          </div>

          {/* Device Selector */}
          <div style={{ display: 'flex', alignItems: 'center', gap: '10px', flex: 1, minWidth: '320px' }}>
            <label style={{ fontWeight: 600, fontSize: '12.5px', color: 'var(--text-secondary)' }}>Output Device:</label>
            <select
              className="select-input"
              value={selectedDeviceId || ''}
              onChange={(e) => onSelectDevice(e.target.value)}
            >
              {devices.map((d) => (
                <option key={d.id} value={d.id}>
                  {d.name} {d.is_default ? '(Default)' : ''}
                </option>
              ))}
            </select>
          </div>

          <div>
            <button className="btn btn-sm" onClick={onResetDefaults} title="Reset all current sliders to flat 0 dB response">
              Reset Controls
            </button>
          </div>
        </div>
      </div>

      {activePreset && (
        <div style={{ fontSize: '11.5px', color: 'var(--text-muted)', marginBottom: '12px', padding: '0 4px' }}>
          <strong>{activePreset.name}:</strong> {activePreset.description}
        </div>
      )}

      {/* Primary Tone Shaping Controls */}
      <div className="grid-3" style={{ marginBottom: '14px' }}>
        {/* Master Output */}
        <div className="panel" style={{ margin: 0 }}>
          <div className="panel-header">Master Level</div>
          <div className="panel-body">
            <div className="control-group">
              <div className="control-label">
                <span>Output Gain</span>
                <span className="control-value">{(config.master_volume * 100).toFixed(0)}%</span>
              </div>
              <input
                type="range"
                className="slider-range"
                min="0"
                max="2"
                step="0.01"
                value={config.master_volume}
                onChange={(e) => handleMasterChange(parseFloat(e.target.value))}
              />
            </div>
            <div style={{ fontSize: '11px', color: 'var(--text-muted)', marginTop: '8px' }}>
              Unity output is 100%. Master level applies smooth parameter ramping to eliminate clicks.
            </div>
          </div>
        </div>

        {/* Bass & Treble */}
        <div className="panel" style={{ margin: 0 }}>
          <div className="panel-header">Tone Balance</div>
          <div className="panel-body">
            <div className="control-group">
              <div className="control-label">
                <span>Bass (Low-Shelf 120 Hz)</span>
                <span className="control-value">{config.bass_gain_db > 0 ? `+${config.bass_gain_db.toFixed(1)}` : config.bass_gain_db.toFixed(1)} dB</span>
              </div>
              <input
                type="range"
                className="slider-range"
                min="-12"
                max="12"
                step="0.5"
                value={config.bass_gain_db}
                onChange={(e) => handleBassChange(parseFloat(e.target.value))}
              />
            </div>

            <div className="control-group" style={{ marginBottom: 0 }}>
              <div className="control-label">
                <span>Treble (High-Shelf 7.5 kHz)</span>
                <span className="control-value">{config.treble_gain_db > 0 ? `+${config.treble_gain_db.toFixed(1)}` : config.treble_gain_db.toFixed(1)} dB</span>
              </div>
              <input
                type="range"
                className="slider-range"
                min="-12"
                max="12"
                step="0.5"
                value={config.treble_gain_db}
                onChange={(e) => handleTrebleChange(parseFloat(e.target.value))}
              />
            </div>
          </div>
        </div>

        {/* Vocal Clarity & Virtual Spatial */}
        <div className="panel" style={{ margin: 0 }}>
          <div className="panel-header">Clarity & Space</div>
          <div className="panel-body">
            <div className="control-group">
              <div className="control-label">
                <span>Vocal Presence (3.0 kHz)</span>
                <span className="control-value">+{config.vocal_clarity_db.toFixed(1)} dB</span>
              </div>
              <input
                type="range"
                className="slider-range"
                min="0"
                max="12"
                step="0.5"
                value={config.vocal_clarity_db}
                onChange={(e) => handleVocalChange(parseFloat(e.target.value))}
              />
            </div>

            <div className="control-group" style={{ marginBottom: 0 }}>
              <div className="control-label">
                <span style={{ display: 'flex', alignItems: 'center', gap: '6px' }}>
                  <input
                    type="checkbox"
                    checked={config.spatial_enabled}
                    onChange={(e) => onChangeConfig({ ...config, spatial_enabled: e.target.checked })}
                  />
                  <span>Virtual Spatial Crossfeed</span>
                </span>
                <span className="control-value">{(config.spatial_width * 100).toFixed(0)}%</span>
              </div>
              <input
                type="range"
                className="slider-range"
                min="0"
                max="1"
                step="0.05"
                disabled={!config.spatial_enabled}
                value={config.spatial_width}
                onChange={(e) => handleSpatialWidthChange(parseFloat(e.target.value))}
              />
            </div>
          </div>
        </div>
      </div>

      {/* 10-Band Graphic Equalizer */}
      <div className="panel">
        <div className="panel-header">
          <span>10-Band Graphic Equalizer</span>
          <div style={{ display: 'flex', alignItems: 'center', gap: '12px' }}>
            <button
              className="btn btn-sm"
              onClick={() => onChangeConfig({ ...config, eq_bands_db: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0] })}
            >
              Flatten Bands
            </button>
            <label style={{ fontSize: '11.5px', display: 'flex', alignItems: 'center', gap: '5px', cursor: 'pointer' }}>
              <input
                type="checkbox"
                checked={config.limiter_enabled}
                onChange={(e) => onChangeConfig({ ...config, limiter_enabled: e.target.checked })}
              />
              <span>Soft-Clipping Protection (Limiter -0.5 dB)</span>
            </label>
          </div>
        </div>

        <div className="panel-body">
          <div className="eq-container">
            {EQ_FREQUENCIES.map((freq, idx) => {
              const val = config.eq_bands_db[idx] || 0;
              return (
                <div key={freq} className="eq-column">
                  <div className="eq-val-badge">{val > 0 ? `+${val.toFixed(1)}` : val.toFixed(1)}</div>
                  <div className="eq-slider-vertical-wrapper">
                    <input
                      type="range"
                      className="eq-slider-vertical"
                      min="-12"
                      max="12"
                      step="0.5"
                      value={val}
                      onChange={(e) => handleEqBandChange(idx, parseFloat(e.target.value))}
                      title={`${freq}: ${val.toFixed(1)} dB`}
                    />
                  </div>
                  <div className="eq-freq-label">{freq}</div>
                </div>
              );
            })}
          </div>

          <div style={{ display: 'flex', justifyContent: 'space-between', marginTop: '10px', fontSize: '11px', color: 'var(--text-muted)' }}>
            <span>Range: ±12.0 dB per octave band</span>
            <span>Real-time IIR second-order biquad peaking filters</span>
          </div>
        </div>
      </div>
    </div>
  );
};
