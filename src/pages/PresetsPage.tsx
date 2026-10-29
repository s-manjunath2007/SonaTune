import React, { useState } from 'react';
import type { Preset, DspConfig } from '../types';

interface PresetsPageProps {
  presets: Preset[];
  activePresetId: string;
  onSelectPreset: (presetId: string) => void;
  onSaveCustomPreset: (preset: Preset) => void;
  onDeleteCustomPreset: (presetId: string) => void;
  currentConfig: DspConfig;
}

export const PresetsPage: React.FC<PresetsPageProps> = ({
  presets,
  activePresetId,
  onSelectPreset,
  onSaveCustomPreset,
  onDeleteCustomPreset,
  currentConfig,
}) => {
  const [newPresetName, setNewPresetName] = useState<string>('');
  const [newPresetDesc, setNewPresetDesc] = useState<string>('');
  const [isCreating, setIsCreating] = useState<boolean>(false);
  const [selectedPresetForView, setSelectedPresetForView] = useState<Preset | null>(
    presets.find((p) => p.id === activePresetId) || presets[0] || null
  );

  const handleCreate = () => {
    if (!newPresetName.trim()) return;
    const newPreset: Preset = {
      id: `custom-${Date.now()}`,
      name: newPresetName.trim(),
      description: newPresetDesc.trim() || 'Custom user preset based on current sound settings.',
      is_builtin: false,
      config: { ...currentConfig },
    };
    onSaveCustomPreset(newPreset);
    setNewPresetName('');
    setNewPresetDesc('');
    setIsCreating(false);
    setSelectedPresetForView(newPreset);
  };

  return (
    <div style={{ maxWidth: '960px', margin: '0 auto' }}>
      <div className="grid-2">
        {/* Preset List */}
        <div className="panel">
          <div className="panel-header">
            <span>Audio Presets Library</span>
            <button className="btn btn-primary btn-sm" onClick={() => setIsCreating(!isCreating)}>
              {isCreating ? 'Cancel' : '+ New Custom Preset'}
            </button>
          </div>

          <div className="panel-body" style={{ padding: 0 }}>
            {isCreating && (
              <div style={{ padding: '12px', backgroundColor: 'var(--bg-surface-alt)', borderBottom: '1px solid var(--border-color)' }}>
                <div style={{ fontWeight: 600, fontSize: '12px', marginBottom: '8px' }}>
                  Save Current Sound Settings as Preset
                </div>
                <div className="control-group">
                  <label className="control-label">Preset Name</label>
                  <input
                    type="text"
                    className="text-input"
                    placeholder="e.g. Acoustic Jazz, Late Night Movie..."
                    value={newPresetName}
                    onChange={(e) => setNewPresetName(e.target.value)}
                  />
                </div>
                <div className="control-group">
                  <label className="control-label">Description (Optional)</label>
                  <input
                    type="text"
                    className="text-input"
                    placeholder="Brief description of sonic characteristics..."
                    value={newPresetDesc}
                    onChange={(e) => setNewPresetDesc(e.target.value)}
                  />
                </div>
                <button
                  className="btn btn-primary btn-sm"
                  disabled={!newPresetName.trim()}
                  onClick={handleCreate}
                >
                  Save Preset
                </button>
              </div>
            )}

            <div style={{ maxHeight: '450px', overflowY: 'auto' }}>
              <table className="data-table">
                <thead>
                  <tr>
                    <th>Name</th>
                    <th>Type</th>
                    <th>Action</th>
                  </tr>
                </thead>
                <tbody>
                  {presets.map((p) => {
                    const isActive = p.id === activePresetId;
                    const isViewing = selectedPresetForView?.id === p.id;
                    return (
                      <tr
                        key={p.id}
                        style={{
                          backgroundColor: isViewing ? 'var(--bg-surface-inset)' : 'transparent',
                          cursor: 'pointer',
                        }}
                        onClick={() => setSelectedPresetForView(p)}
                      >
                        <td>
                          <strong>{p.name}</strong>
                          {isActive && <span className="status-badge active" style={{ marginLeft: '6px' }}>Active</span>}
                        </td>
                        <td>
                          <span className="status-badge">{p.is_builtin ? 'Factory' : 'Custom'}</span>
                        </td>
                        <td>
                          <div style={{ display: 'flex', gap: '4px' }}>
                            <button
                              className="btn btn-sm"
                              onClick={(e) => {
                                e.stopPropagation();
                                onSelectPreset(p.id);
                              }}
                            >
                              Apply
                            </button>
                            {!p.is_builtin && (
                              <button
                                className="btn btn-sm btn-danger"
                                onClick={(e) => {
                                  e.stopPropagation();
                                  onDeleteCustomPreset(p.id);
                                  if (selectedPresetForView?.id === p.id) {
                                    setSelectedPresetForView(presets[0]);
                                  }
                                }}
                              >
                                Delete
                              </button>
                            )}
                          </div>
                        </td>
                      </tr>
                    );
                  })}
                </tbody>
              </table>
            </div>
          </div>
        </div>

        {/* Preset Inspector */}
        <div className="panel">
          <div className="panel-header">
            <span>Preset Parameters</span>
            {selectedPresetForView && (
              <button
                className="btn btn-primary btn-sm"
                onClick={() => onSelectPreset(selectedPresetForView.id)}
              >
                Apply to Sound Engine
              </button>
            )}
          </div>
          <div className="panel-body">
            {selectedPresetForView ? (
              <div>
                <h3 style={{ fontSize: '15px', marginBottom: '4px' }}>{selectedPresetForView.name}</h3>
                <p style={{ fontSize: '12px', color: 'var(--text-secondary)', marginBottom: '16px' }}>
                  {selectedPresetForView.description}
                </p>

                <div style={{ display: 'flex', flexDirection: 'column', gap: '8px', fontSize: '12px' }}>
                  <div style={{ display: 'flex', justifyContent: 'space-between', padding: '6px 0', borderBottom: '1px solid var(--border-color-subtle)' }}>
                    <span>Bass Gain (Low-Shelf 120 Hz):</span>
                    <strong>{selectedPresetForView.config.bass_gain_db.toFixed(1)} dB</strong>
                  </div>
                  <div style={{ display: 'flex', justifyContent: 'space-between', padding: '6px 0', borderBottom: '1px solid var(--border-color-subtle)' }}>
                    <span>Treble Gain (High-Shelf 7.5 kHz):</span>
                    <strong>{selectedPresetForView.config.treble_gain_db.toFixed(1)} dB</strong>
                  </div>
                  <div style={{ display: 'flex', justifyContent: 'space-between', padding: '6px 0', borderBottom: '1px solid var(--border-color-subtle)' }}>
                    <span>Vocal Clarity Boost (3.0 kHz):</span>
                    <strong>+{selectedPresetForView.config.vocal_clarity_db.toFixed(1)} dB</strong>
                  </div>
                  <div style={{ display: 'flex', justifyContent: 'space-between', padding: '6px 0', borderBottom: '1px solid var(--border-color-subtle)' }}>
                    <span>Virtual Spatial Width:</span>
                    <strong>{(selectedPresetForView.config.spatial_width * 100).toFixed(0)}%</strong>
                  </div>
                  <div style={{ display: 'flex', justifyContent: 'space-between', padding: '6px 0', borderBottom: '1px solid var(--border-color-subtle)' }}>
                    <span>Soft Limiter:</span>
                    <strong>{selectedPresetForView.config.limiter_enabled ? 'Active (-0.5 dB)' : 'Disabled'}</strong>
                  </div>
                </div>

                <div style={{ marginTop: '16px' }}>
                  <div style={{ fontWeight: 600, fontSize: '12px', marginBottom: '6px' }}>EQ Curve Offset:</div>
                  <div style={{ display: 'flex', gap: '4px', fontSize: '10.5px', fontFamily: 'Consolas, monospace', overflowX: 'auto', paddingBottom: '4px' }}>
                    {selectedPresetForView.config.eq_bands_db.map((b, i) => (
                      <span key={i} style={{ padding: '2px 6px', background: 'var(--bg-surface-inset)', borderRadius: '2px' }}>
                        {b > 0 ? `+${b.toFixed(1)}` : b.toFixed(1)}
                      </span>
                    ))}
                  </div>
                </div>
              </div>
            ) : (
              <div style={{ color: 'var(--text-muted)', textAlign: 'center', padding: '20px' }}>
                Select a preset to inspect parameters
              </div>
            )}
          </div>
        </div>
      </div>
    </div>
  );
};
