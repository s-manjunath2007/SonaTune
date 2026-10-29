import React, { useState } from 'react';
import type { AudioDeviceInfo, LiveAudioStatus, Preset, DspConfig } from '../types';
import { apiStartLiveAudio, apiStopLiveAudio } from '../api';

interface DevicesPageProps {
  devices: AudioDeviceInfo[];
  selectedDeviceId?: string;
  onSelectDevice: (deviceId: string) => void;
  liveStatus: LiveAudioStatus;
  onUpdateLiveStatus: (status: LiveAudioStatus) => void;
  presets: Preset[];
  deviceProfiles: Record<string, DspConfig>;
  onAssignDeviceProfile: (deviceId: string, config: DspConfig) => void;
  onRefreshDevices: () => void;
}

export const DevicesPage: React.FC<DevicesPageProps> = ({
  devices,
  selectedDeviceId,
  onSelectDevice,
  liveStatus,
  onUpdateLiveStatus,
  presets,
  deviceProfiles,
  onAssignDeviceProfile,
  onRefreshDevices,
}) => {
  const [isStartingRoute, setIsStartingRoute] = useState<boolean>(false);
  const [routeError, setRouteError] = useState<string | null>(null);

  const handleToggleLive = async () => {
    setIsStartingRoute(true);
    setRouteError(null);
    try {
      if (liveStatus.state === 'Running') {
        const st = await apiStopLiveAudio();
        onUpdateLiveStatus(st);
      } else {
        const st = await apiStartLiveAudio(undefined, selectedDeviceId);
        onUpdateLiveStatus(st);
      }
    } catch (err: any) {
      setRouteError(typeof err === 'string' ? err : err.message || 'Routing failed');
    } finally {
      setIsStartingRoute(false);
    }
  };

  return (
    <div style={{ maxWidth: '960px', margin: '0 auto' }}>
      {/* Real Windows Devices */}
      <div className="panel">
        <div className="panel-header">
          <span>Detected Windows Audio Output Devices</span>
          <button className="btn btn-sm" onClick={onRefreshDevices}>
            Refresh Device List
          </button>
        </div>
        <div className="panel-body" style={{ padding: 0 }}>
          <table className="data-table">
            <thead>
              <tr>
                <th style={{ width: '40px' }}>Active</th>
                <th>Device Name (Reported by Windows)</th>
                <th>Channels</th>
                <th>Default Sample Rate</th>
                <th>Device Profile</th>
              </tr>
            </thead>
            <tbody>
              {devices.map((d) => {
                const isSelected = (selectedDeviceId ? selectedDeviceId === d.id : d.is_default);
                const hasAssignedProfile = deviceProfiles[d.id] !== undefined;

                return (
                  <tr key={d.id} style={{ backgroundColor: isSelected ? 'var(--bg-surface-inset)' : 'transparent' }}>
                    <td>
                      <input
                        type="radio"
                        name="active_device"
                        checked={isSelected}
                        onChange={() => onSelectDevice(d.id)}
                      />
                    </td>
                    <td>
                      <strong>{d.name}</strong>
                      {d.is_default && <span className="status-badge" style={{ marginLeft: '8px' }}>Default</span>}
                    </td>
                    <td>{d.max_channels} Ch</td>
                    <td>{d.default_sample_rate} Hz</td>
                    <td>
                      <select
                        className="select-input"
                        style={{ width: '160px', padding: '2px 6px', fontSize: '11.5px' }}
                        value={hasAssignedProfile ? 'custom' : 'headphones'}
                        onChange={(e) => {
                          const p = presets.find((pr) => pr.id === e.target.value);
                          if (p) {
                            onAssignDeviceProfile(d.id, p.config);
                          }
                        }}
                      >
                        <option value="headphones">Headphones Profile</option>
                        <option value="speakers">Speakers Profile</option>
                        <option value="balanced">Balanced Profile</option>
                        <option value="music">Music Profile</option>
                      </select>
                    </td>
                  </tr>
                );
              })}
              {devices.length === 0 && (
                <tr>
                  <td colSpan={5} style={{ textAlign: 'center', padding: '16px', color: 'var(--text-muted)' }}>
                    No output audio devices detected by WASAPI/CPAL.
                  </td>
                </tr>
              )}
            </tbody>
          </table>
        </div>
      </div>

      {/* System-Wide Live Audio Routing Proof of Concept */}
      <div className="panel">
        <div className="panel-header">
          <span>Windows Live System-Wide Audio Routing (WASAPI Engine)</span>
          <span className={`status-badge ${liveStatus.state === 'Running' ? 'active' : liveStatus.state === 'Error' ? 'error' : ''}`}>
            {liveStatus.state.toUpperCase()}
          </span>
        </div>
        <div className="panel-body">
          <div style={{ marginBottom: '14px', fontSize: '12.5px', lineHeight: '1.5' }}>
            <p>
              To enhance audio from any Windows application (Chrome, Spotify, VLC, games), Windows requires a virtual audio bridge to separate application playback from physical hardware rendering. This ensures genuine processing while preventing destructive acoustic feedback loops.
            </p>
          </div>

          <div style={{ marginBottom: '16px', padding: '12px 14px', backgroundColor: 'var(--bg-surface)', border: '1px solid var(--border-color)', borderRadius: '3px' }}>
            <div style={{ fontWeight: 600, marginBottom: '8px', fontSize: '13px' }}>
              Step-by-Step Setup Workflow:
            </div>
            <ol style={{ paddingLeft: '20px', fontSize: '12px', lineHeight: '1.6', margin: 0, color: 'var(--text-secondary)' }}>
              <li>
                <strong>Install Virtual Driver:</strong> Install a standard Windows virtual audio endpoint such as <em>VB-CABLE Driver</em> (VB-Audio Software, free/donationware, 64-bit Windows 10/11).
              </li>
              <li>
                <strong>Set Windows Default Output:</strong> Open <em>Windows Sound Settings</em> (or Taskbar Volume icon) and select <code>CABLE Input (VB-Audio Virtual Cable)</code> as your system playback device.
              </li>
              <li>
                <strong>Select Physical Hardware in SonaTune:</strong> In the device list above, choose your actual physical headphones or speakers as the active output device.
              </li>
              <li>
                <strong>Engage Live Enhancement:</strong> Click <em>"Start Live Audio Routing"</em> below. SonaTune captures the audio from the virtual endpoint, processes it in real time through the 10-band EQ, clarity shaper, and soft limiter, and renders it directly to your physical headphones/speakers.
              </li>
              <li>
                <strong>Restoring Standard Output:</strong> When live enhancement is stopped, switch Windows default output device back to your physical speakers/headphones. SonaTune never modifies your Windows default device automatically.
              </li>
            </ol>
          </div>

          <div className="grid-2" style={{ marginBottom: '16px' }}>
            <div style={{ padding: '10px 12px', backgroundColor: 'var(--bg-surface-alt)', border: '1px solid var(--border-color-subtle)', borderRadius: '3px' }}>
              <div style={{ fontWeight: 600, marginBottom: '6px' }}>Architecture & Routing Pipeline:</div>
              <ul style={{ paddingLeft: '18px', fontSize: '12px', color: 'var(--text-secondary)' }}>
                <li>1. Apps play audio to Default Virtual Endpoint</li>
                <li>2. SonaTune captures stream with WASAPI</li>
                <li>3. Real-time Biquad DSP & Limiter applied</li>
                <li>4. Rendered directly to selected hardware output</li>
              </ul>
            </div>

            <div style={{ padding: '10px 12px', backgroundColor: 'var(--bg-surface-alt)', border: '1px solid var(--border-color-subtle)', borderRadius: '3px' }}>
              <div style={{ fontWeight: 600, marginBottom: '6px' }}>Diagnostic Status:</div>
              <div style={{ fontSize: '12px', color: 'var(--text-secondary)', display: 'flex', flexDirection: 'column', gap: '4px' }}>
                <div>Virtual Endpoint Installed: <strong>{liveStatus.is_virtual_driver_present ? 'Yes (Detected)' : 'Standard Only'}</strong></div>
                <div>Feedback Protection: <strong>Engaged (Direct same-device loopback blocked)</strong></div>
                <div>Estimated Loop Latency: <strong>{liveStatus.estimated_latency_ms.toFixed(1)} ms</strong></div>
              </div>
            </div>
          </div>

          {routeError && (
            <div style={{ marginBottom: '12px', padding: '8px 12px', backgroundColor: '#fef2f2', border: '1px solid #f87171', borderRadius: '3px', color: '#991b1b', fontSize: '12px' }}>
              {routeError}
            </div>
          )}

          <div style={{ display: 'flex', gap: '10px', alignItems: 'center' }}>
            <button
              className={`btn ${liveStatus.state === 'Running' ? 'btn-danger' : 'btn-primary'}`}
              disabled={isStartingRoute}
              onClick={handleToggleLive}
            >
              {liveStatus.state === 'Running' ? 'Stop Live Audio Route' : 'Start Live Audio Routing'}
            </button>
            <span style={{ fontSize: '12px', color: 'var(--text-muted)' }}>
              {liveStatus.message}
            </span>
          </div>
        </div>
      </div>
    </div>
  );
};
