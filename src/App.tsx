import React, { useState, useEffect } from 'react';
import './App.css';
import type {
  AppSettings,
  DspConfig,
  Preset,
  AudioDeviceInfo,
  LiveAudioStatus,
  MediaToolInfo,
} from './types';
import {
  apiGetSettings,
  apiSaveSettings,
  apiGetPresets,
  apiSaveCustomPreset,
  apiDeleteCustomPreset,
  apiGetAudioDevices,
  apiCheckMediaTools,
  apiGetLiveAudioStatus,
  defaultDspConfig,
  defaultSettings,
} from './api';

import { MenuBar } from './components/MenuBar';
import { StatusBar } from './components/StatusBar';
import { SoundPage } from './pages/SoundPage';
import { AudioFilesPage } from './pages/AudioFilesPage';
import { VideoPage } from './pages/VideoPage';
import { DevicesPage } from './pages/DevicesPage';
import { PresetsPage } from './pages/PresetsPage';
import { SettingsPage } from './pages/SettingsPage';

export const App: React.FC = () => {
  const [settings, setSettings] = useState<AppSettings>(defaultSettings);
  const [presets, setPresets] = useState<Preset[]>([]);
  const [devices, setDevices] = useState<AudioDeviceInfo[]>([]);
  const [selectedDeviceId, setSelectedDeviceId] = useState<string | undefined>(undefined);
  const [liveStatus, setLiveStatus] = useState<LiveAudioStatus>({
    state: 'Stopped',
    sample_rate: 48000,
    channels: 2,
    estimated_latency_ms: 0,
    is_virtual_driver_present: false,
    feedback_protection_engaged: true,
    message: 'Live audio routing idle.',
  });
  const [mediaTools, setMediaTools] = useState<MediaToolInfo>({
    ffmpeg_available: false,
    ffprobe_available: false,
  });

  const [activePage, setActivePage] = useState<string>('sound');
  const [showAboutModal, setShowAboutModal] = useState<boolean>(false);

  // Initialize data on mount
  useEffect(() => {
    async function init() {
      try {
        const loadedSettings = await apiGetSettings();
        setSettings(loadedSettings);
        if (loadedSettings.last_page) {
          setActivePage(loadedSettings.last_page);
        }

        const loadedPresets = await apiGetPresets();
        setPresets(loadedPresets);

        const loadedDevices = await apiGetAudioDevices();
        setDevices(loadedDevices);
        const defDev = loadedDevices.find((d) => d.is_default);
        if (defDev) {
          setSelectedDeviceId(loadedSettings.selected_device_id || defDev.id);
        }

        const tools = await apiCheckMediaTools();
        setMediaTools(tools);

        const live = await apiGetLiveAudioStatus();
        setLiveStatus(live);
      } catch (e) {
        console.error('Initialization error:', e);
      }
    }
    init();
  }, []);

  // Sync theme with document
  useEffect(() => {
    document.documentElement.setAttribute('data-theme', settings.theme);
  }, [settings.theme]);

  // Keyboard navigation shortcuts
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement) {
        return;
      }

      if (e.ctrlKey && e.key === '1') {
        e.preventDefault();
        navigateTo('sound');
      } else if (e.ctrlKey && e.key === '2') {
        e.preventDefault();
        navigateTo('audio');
      } else if (e.ctrlKey && e.key === '3') {
        e.preventDefault();
        navigateTo('video');
      } else if (e.ctrlKey && e.key === '4') {
        e.preventDefault();
        navigateTo('devices');
      } else if (e.ctrlKey && e.key === '5') {
        e.preventDefault();
        navigateTo('presets');
      } else if (e.ctrlKey && e.key === '6') {
        e.preventDefault();
        navigateTo('settings');
      } else if (e.ctrlKey && e.key === ',') {
        e.preventDefault();
        navigateTo('settings');
      } else if (e.key === ' ' && !e.repeat) {
        e.preventDefault();
        handleToggleMasterEnhancement();
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [settings]);

  const navigateTo = (page: string) => {
    setActivePage(page);
    const updated = { ...settings, last_page: page };
    setSettings(updated);
    apiSaveSettings(updated);
  };

  const handleToggleMasterEnhancement = () => {
    const newEnabled = !settings.current_config.enabled;
    const updatedConfig: DspConfig = {
      ...settings.current_config,
      enabled: newEnabled,
    };
    const updatedSettings: AppSettings = {
      ...settings,
      current_config: updatedConfig,
    };
    setSettings(updatedSettings);
    apiSaveSettings(updatedSettings);
  };

  const handleChangeConfig = (newConfig: DspConfig) => {
    const updatedSettings: AppSettings = {
      ...settings,
      current_config: newConfig,
    };
    setSettings(updatedSettings);
    apiSaveSettings(updatedSettings);
  };

  const handleSelectPreset = (presetId: string) => {
    const p = presets.find((pr) => pr.id === presetId);
    if (p) {
      const updatedConfig: DspConfig = {
        ...p.config,
        enabled: settings.current_config.enabled, // preserve master switch
      };
      const updatedSettings: AppSettings = {
        ...settings,
        active_preset_id: presetId,
        current_config: updatedConfig,
      };
      setSettings(updatedSettings);
      apiSaveSettings(updatedSettings);
    }
  };

  const handleSelectDevice = (deviceId: string) => {
    setSelectedDeviceId(deviceId);

    // If device has a saved profile, apply it
    let nextConfig = settings.current_config;
    if (settings.device_profiles[deviceId]) {
      nextConfig = { ...settings.device_profiles[deviceId] };
    }

    const updatedSettings: AppSettings = {
      ...settings,
      selected_device_id: deviceId,
      current_config: nextConfig,
    };
    setSettings(updatedSettings);
    apiSaveSettings(updatedSettings);
  };

  const handleAssignDeviceProfile = (deviceId: string, config: DspConfig) => {
    const updatedProfiles = {
      ...settings.device_profiles,
      [deviceId]: config,
    };
    const updatedSettings: AppSettings = {
      ...settings,
      device_profiles: updatedProfiles,
    };
    setSettings(updatedSettings);
    apiSaveSettings(updatedSettings);
  };

  const handleSaveCustomPreset = async (preset: Preset) => {
    try {
      const updatedList = await apiSaveCustomPreset(preset);
      setPresets(updatedList);
      handleSelectPreset(preset.id);
    } catch (e) {
      console.error(e);
    }
  };

  const handleDeleteCustomPreset = async (presetId: string) => {
    try {
      const updatedList = await apiDeleteCustomPreset(presetId);
      setPresets(updatedList);
      if (settings.active_preset_id === presetId) {
        handleSelectPreset('balanced');
      }
    } catch (e) {
      console.error(e);
    }
  };

  const handleResetDefaults = () => {
    handleChangeConfig(defaultDspConfig);
  };

  const handleUpdateExportDir = (dir: string) => {
    const updatedSettings: AppSettings = {
      ...settings,
      last_export_dir: dir,
    };
    setSettings(updatedSettings);
    apiSaveSettings(updatedSettings);
  };

  const activePreset = presets.find((p) => p.id === settings.active_preset_id);
  const activeDevice = devices.find((d) => (selectedDeviceId ? d.id === selectedDeviceId : d.is_default));

  return (
    <div className="app-container">
      {/* Windows Application Menu Bar */}
      <MenuBar
        onNavigate={navigateTo}
        onResetDsp={handleResetDefaults}
        onShowAbout={() => setShowAboutModal(true)}
        isEnhancementActive={settings.current_config.enabled}
        onToggleEnhancement={handleToggleMasterEnhancement}
      />

      {/* Main Header & Tab Navigation */}
      <div className="nav-header">
        <div className="brand-section">
          <div className="brand-logo">S</div>
          <div className="brand-title">SonaTune</div>
        </div>

        <nav className="nav-tabs">
          <button
            className={`nav-tab ${activePage === 'sound' ? 'active' : ''}`}
            onClick={() => navigateTo('sound')}
          >
            Sound
          </button>
          <button
            className={`nav-tab ${activePage === 'audio' ? 'active' : ''}`}
            onClick={() => navigateTo('audio')}
          >
            Audio Files
          </button>
          <button
            className={`nav-tab ${activePage === 'video' ? 'active' : ''}`}
            onClick={() => navigateTo('video')}
          >
            Video
          </button>
          <button
            className={`nav-tab ${activePage === 'devices' ? 'active' : ''}`}
            onClick={() => navigateTo('devices')}
          >
            Devices
          </button>
          <button
            className={`nav-tab ${activePage === 'presets' ? 'active' : ''}`}
            onClick={() => navigateTo('presets')}
          >
            Presets
          </button>
          <button
            className={`nav-tab ${activePage === 'settings' ? 'active' : ''}`}
            onClick={() => navigateTo('settings')}
          >
            Settings
          </button>
        </nav>

        <div className="header-actions">
          {/* Master Enhancement Button */}
          <button
            className={`master-toggle-btn ${settings.current_config.enabled ? 'active' : ''}`}
            onClick={handleToggleMasterEnhancement}
            title="Toggle DSP Enhancement bypass (Spacebar)"
          >
            <span className="indicator-dot" />
            <span>{settings.current_config.enabled ? 'Enhancement Active' : 'DSP Bypassed'}</span>
          </button>
        </div>
      </div>

      {/* Main Workspace Body */}
      <main className="main-workspace">
        {activePage === 'sound' && (
          <SoundPage
            config={settings.current_config}
            onChangeConfig={handleChangeConfig}
            presets={presets}
            activePresetId={settings.active_preset_id}
            onSelectPreset={handleSelectPreset}
            devices={devices}
            selectedDeviceId={selectedDeviceId}
            onSelectDevice={handleSelectDevice}
            onResetDefaults={handleResetDefaults}
          />
        )}

        {activePage === 'audio' && (
          <AudioFilesPage
            currentConfig={settings.current_config}
            presets={presets}
            lastExportDir={settings.last_export_dir}
            onUpdateExportDir={handleUpdateExportDir}
          />
        )}

        {activePage === 'video' && (
          <VideoPage
            currentConfig={settings.current_config}
            presets={presets}
            mediaTools={mediaTools}
            lastExportDir={settings.last_export_dir}
            onUpdateExportDir={handleUpdateExportDir}
          />
        )}

        {activePage === 'devices' && (
          <DevicesPage
            devices={devices}
            selectedDeviceId={selectedDeviceId}
            onSelectDevice={handleSelectDevice}
            liveStatus={liveStatus}
            onUpdateLiveStatus={setLiveStatus}
            presets={presets}
            deviceProfiles={settings.device_profiles}
            onAssignDeviceProfile={handleAssignDeviceProfile}
            onRefreshDevices={async () => {
              const devs = await apiGetAudioDevices();
              setDevices(devs);
            }}
          />
        )}

        {activePage === 'presets' && (
          <PresetsPage
            presets={presets}
            activePresetId={settings.active_preset_id}
            onSelectPreset={handleSelectPreset}
            onSaveCustomPreset={handleSaveCustomPreset}
            onDeleteCustomPreset={handleDeleteCustomPreset}
            currentConfig={settings.current_config}
          />
        )}

        {activePage === 'settings' && (
          <SettingsPage
            settings={settings}
            onChangeSettings={(newSt) => {
              setSettings(newSt);
              apiSaveSettings(newSt);
            }}
            mediaTools={mediaTools}
          />
        )}
      </main>

      {/* Application Status Bar */}
      <StatusBar
        activeDevice={activeDevice}
        enhancementEnabled={settings.current_config.enabled}
        liveStatus={liveStatus}
        mediaTools={mediaTools}
        activePresetName={activePreset ? activePreset.name : 'Balanced'}
      />

      {/* About SonaTune Modal */}
      {showAboutModal && (
        <div className="modal-overlay" onClick={() => setShowAboutModal(false)}>
          <div className="modal-dialog" onClick={(e) => e.stopPropagation()}>
            <div className="modal-header">
              <span>About SonaTune</span>
              <button
                style={{ background: 'none', border: 'none', cursor: 'pointer', fontSize: '14px' }}
                onClick={() => setShowAboutModal(false)}
              >
                ✕
              </button>
            </div>
            <div className="modal-body">
              <h3 style={{ fontSize: '15px', marginBottom: '8px' }}>SonaTune Desktop</h3>
              <p style={{ color: 'var(--text-secondary)', marginBottom: '12px' }}>
                Professional Windows desktop audio enhancement utility designed for headphones, earbuds, and speakers.
              </p>
              <ul style={{ paddingLeft: '18px', color: 'var(--text-secondary)', lineHeight: '1.6' }}>
                <li><strong>Native DSP:</strong> Biquad peaking, shelf filtering, and soft limiter.</li>
                <li><strong>Audio File Studio:</strong> A/B loudness-matched comparison and lossless export.</li>
                <li><strong>Video Enhancement:</strong> Stream-copy lossless video remuxing with FFmpeg.</li>
                <li><strong>Local & Offline:</strong> Runs 100% locally on your machine with zero external network dependencies.</li>
              </ul>
            </div>
            <div className="modal-footer">
              <button className="btn btn-primary" onClick={() => setShowAboutModal(false)}>
                Close
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
};
export default App;
