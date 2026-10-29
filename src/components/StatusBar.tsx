import React from 'react';
import type { AudioDeviceInfo, LiveAudioStatus, MediaToolInfo } from '../types';

interface StatusBarProps {
  activeDevice?: AudioDeviceInfo;
  enhancementEnabled: boolean;
  liveStatus: LiveAudioStatus;
  mediaTools: MediaToolInfo;
  activePresetName: string;
}

export const StatusBar: React.FC<StatusBarProps> = ({
  activeDevice,
  enhancementEnabled,
  liveStatus,
  mediaTools,
  activePresetName,
}) => {
  return (
    <div className="status-bar">
      <div className="status-left">
        <div className="status-item">
          <span>DSP Status:</span>
          <span className={`status-badge ${enhancementEnabled ? 'active' : ''}`}>
            {enhancementEnabled ? 'Active' : 'Bypassed'}
          </span>
        </div>

        <div className="status-item">
          <span>Preset:</span>
          <span style={{ fontWeight: 600, color: 'var(--text-primary)' }}>{activePresetName}</span>
        </div>

        <div className="status-item">
          <span>Output:</span>
          <span style={{ color: 'var(--text-primary)' }}>
            {activeDevice ? activeDevice.name : 'Detecting audio device...'}
          </span>
        </div>
      </div>

      <div className="status-right">
        {activeDevice && (
          <div className="status-item">
            <span>{activeDevice.default_sample_rate} Hz • {activeDevice.max_channels} Ch</span>
          </div>
        )}

        <div className="status-item">
          <span>Live Routing:</span>
          <span className={`status-badge ${liveStatus.state === 'Running' ? 'active' : liveStatus.state === 'Error' ? 'error' : ''}`}>
            {liveStatus.state}
          </span>
        </div>

        <div className="status-item">
          <span>FFmpeg:</span>
          <span className={`status-badge ${mediaTools.ffmpeg_available ? 'active' : ''}`}>
            {mediaTools.ffmpeg_available ? 'Ready' : 'Not Found'}
          </span>
        </div>
      </div>
    </div>
  );
};
