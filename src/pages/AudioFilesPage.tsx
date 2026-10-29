import React, { useState, useEffect } from 'react';
import { open } from '@tauri-apps/plugin-dialog';
import type { DspConfig, Preset, AudioMetadata, AudioProcessingResult, PlayerState } from '../types';
import {
  apiInspectAudio,
  apiProcessAudio,
  apiCancelAudioProcessing,
  apiPlayerLoad,
  apiPlayerPlay,
  apiPlayerPause,
  apiPlayerSeek,
  apiPlayerSetAB,
  apiPlayerSetLoudnessMatch,
  apiPlayerGetState,
} from '../api';

interface AudioFilesPageProps {
  currentConfig: DspConfig;
  presets: Preset[];
  lastExportDir?: string | null;
  onUpdateExportDir?: (dir: string) => void;
}

export const AudioFilesPage: React.FC<AudioFilesPageProps> = ({
  currentConfig,
  presets,
  lastExportDir,
  onUpdateExportDir,
}) => {
  const [filePath, setFilePath] = useState<string>('');
  const [metadata, setMetadata] = useState<AudioMetadata | null>(null);
  const [isLoadingFile, setIsLoadingFile] = useState<boolean>(false);
  const [errorMsg, setErrorMsg] = useState<string | null>(null);

  // Player state
  const [playerState, setPlayerState] = useState<PlayerState>({
    is_playing: false,
    current_time_seconds: 0,
    total_duration_seconds: 0,
    a_b_mode: 'processed',
    loudness_matched: true,
    file_loaded: false,
  });

  // Export settings & state
  const [selectedPresetId, setSelectedPresetId] = useState<string>('balanced');
  const [outputFileName, setOutputFileName] = useState<string>('');
  const [outputDir, setOutputDir] = useState<string>(lastExportDir || '');
  const [exportFormat, setExportFormat] = useState<string>('wav');
  const [isProcessing, setIsProcessing] = useState<boolean>(false);
  const [exportResult, setExportResult] = useState<AudioProcessingResult | null>(null);

  useEffect(() => {
    if (lastExportDir && !outputDir) {
      setOutputDir(lastExportDir);
    }
  }, [lastExportDir]);

  // Poll player state while playing
  useEffect(() => {
    let interval: number | undefined;
    if (playerState.is_playing) {
      interval = window.setInterval(async () => {
        try {
          const st = await apiPlayerGetState();
          setPlayerState(st);
        } catch (_) {}
      }, 200);
    }
    return () => clearInterval(interval);
  }, [playerState.is_playing]);

  const handleInspectFile = async (path: string) => {
    if (!path.trim()) return;
    setIsLoadingFile(true);
    setErrorMsg(null);
    setExportResult(null);

    // If output directory is not yet populated, derive it from input file location
    const fileFolder = path.substring(0, Math.max(path.lastIndexOf('\\'), path.lastIndexOf('/')));
    if (!outputDir && fileFolder) {
      setOutputDir(fileFolder);
      onUpdateExportDir?.(fileFolder);
    }

    try {
      const meta = await apiInspectAudio(path.trim());
      setMetadata(meta);

      // Default export filename: name_enhanced.wav
      const baseName = meta.file_name.substring(0, meta.file_name.lastIndexOf('.')) || meta.file_name;
      setOutputFileName(`${baseName}_enhanced.${exportFormat}`);

      // Load into audio preview player
      const state = await apiPlayerLoad(path.trim());
      setPlayerState(state);
    } catch (err: any) {
      setErrorMsg(typeof err === 'string' ? err : err.message || 'Failed to inspect audio file');
    } finally {
      setIsLoadingFile(false);
    }
  };

  const handleBrowseFile = async () => {
    try {
      const selected = await open({
        multiple: false,
        directory: false,
        filters: [
          {
            name: 'Audio Files (*.wav, *.mp3, *.flac, *.aac, *.m4a, *.ogg)',
            extensions: ['wav', 'mp3', 'flac', 'aac', 'm4a', 'ogg'],
          },
          {
            name: 'All Files (*.*)',
            extensions: ['*'],
          },
        ],
      });
      if (selected && typeof selected === 'string') {
        setFilePath(selected);
        handleInspectFile(selected);
      }
    } catch (err: any) {
      console.error('File dialog error:', err);
    }
  };

  const handleBrowseOutputDir = async () => {
    try {
      const selected = await open({
        directory: true,
        multiple: false,
        defaultPath: outputDir || undefined,
      });
      if (selected && typeof selected === 'string') {
        setOutputDir(selected);
        onUpdateExportDir?.(selected);
      }
    } catch (err: any) {
      console.error('Directory dialog error:', err);
    }
  };

  const handlePlayPause = async () => {
    try {
      if (playerState.is_playing) {
        const st = await apiPlayerPause();
        setPlayerState(st);
      } else {
        const st = await apiPlayerPlay();
        setPlayerState(st);
      }
    } catch (err: any) {
      setErrorMsg(typeof err === 'string' ? err : err.message);
    }
  };

  const handleSeek = async (val: number) => {
    try {
      const st = await apiPlayerSeek(val);
      setPlayerState(st);
    } catch (_) {}
  };

  const handleABToggle = async (mode: 'original' | 'processed') => {
    try {
      const st = await apiPlayerSetAB(mode);
      setPlayerState(st);
    } catch (_) {}
  };

  const handleLoudnessMatchToggle = async (enabled: boolean) => {
    try {
      const gain = exportResult ? exportResult.loudness_gain_factor : 1.0;
      const st = await apiPlayerSetLoudnessMatch(enabled, gain);
      setPlayerState(st);
    } catch (_) {}
  };

  const handleExport = async () => {
    if (!metadata) return;
    const destPath = `${outputDir.replace(/\\$/, '')}\\${outputFileName}`;

    if (destPath === metadata.file_path) {
      setErrorMsg('Destination path cannot overwrite the original source file.');
      return;
    }

    setIsProcessing(true);
    setErrorMsg(null);
    setExportResult(null);

    const activePreset = presets.find((p) => p.id === selectedPresetId);
    const configToApply = activePreset ? activePreset.config : currentConfig;

    try {
      const result = await apiProcessAudio(metadata.file_path, destPath, configToApply);
      setExportResult(result);
      // Auto-update preview player loudness matching factor
      await apiPlayerSetLoudnessMatch(playerState.loudness_matched, result.loudness_gain_factor);
    } catch (err: any) {
      setErrorMsg(typeof err === 'string' ? err : err.message || 'Export failed');
    } finally {
      setIsProcessing(false);
    }
  };

  const handleCancelExport = async () => {
    await apiCancelAudioProcessing();
    setIsProcessing(false);
  };

  const formatTime = (secs: number) => {
    const m = Math.floor(secs / 60);
    const s = Math.floor(secs % 60);
    return `${m}:${s.toString().padStart(2, '0')}`;
  };

  return (
    <div style={{ maxWidth: '960px', margin: '0 auto' }}>
      {/* File Selection Bar */}
      <div className="panel">
        <div className="panel-header">Audio Source Selection</div>
        <div className="panel-body">
          <div style={{ display: 'flex', gap: '8px', alignItems: 'center' }}>
            <input
              type="text"
              className="text-input"
              placeholder="Enter absolute audio file path (WAV, MP3, FLAC, AAC, M4A, OGG)..."
              value={filePath}
              onChange={(e) => setFilePath(e.target.value)}
              onKeyDown={(e) => e.key === 'Enter' && handleInspectFile(filePath)}
            />
            <button
              className="btn btn-secondary"
              onClick={handleBrowseFile}
              title="Browse computer for audio file"
            >
              Browse...
            </button>
            <button
              className="btn btn-primary"
              disabled={isLoadingFile || !filePath.trim()}
              onClick={() => handleInspectFile(filePath)}
            >
              {isLoadingFile ? 'Inspecting...' : 'Load & Inspect'}
            </button>
          </div>

          {errorMsg && (
            <div style={{ marginTop: '10px', padding: '8px 12px', backgroundColor: '#fef2f2', border: '1px solid #f87171', borderRadius: '3px', color: '#991b1b', fontSize: '12px' }}>
              <strong>Error:</strong> {errorMsg}
            </div>
          )}
        </div>
      </div>

      {metadata && (
        <>
          {/* Metadata & Audio Characteristics */}
          <div className="grid-2">
            <div className="panel">
              <div className="panel-header">Media Properties</div>
              <div className="panel-body" style={{ padding: 0 }}>
                <table className="data-table">
                  <tbody>
                    <tr>
                      <td style={{ width: '130px', fontWeight: 600 }}>File Name</td>
                      <td style={{ wordBreak: 'break-all' }}>{metadata.file_name}</td>
                    </tr>
                    <tr>
                      <td style={{ fontWeight: 600 }}>Duration</td>
                      <td>{formatTime(metadata.duration_seconds)} ({metadata.duration_seconds.toFixed(2)}s)</td>
                    </tr>
                    <tr>
                      <td style={{ fontWeight: 600 }}>Audio Format</td>
                      <td>{metadata.format}</td>
                    </tr>
                    <tr>
                      <td style={{ fontWeight: 600 }}>Sample Rate</td>
                      <td>{metadata.sample_rate} Hz</td>
                    </tr>
                    <tr>
                      <td style={{ fontWeight: 600 }}>Channels</td>
                      <td>{metadata.channels === 1 ? '1 (Mono)' : metadata.channels === 2 ? '2 (Stereo)' : `${metadata.channels} Channels`}</td>
                    </tr>
                    <tr>
                      <td style={{ fontWeight: 600 }}>File Size</td>
                      <td>{(metadata.file_size_bytes / (1024 * 1024)).toFixed(2)} MB</td>
                    </tr>
                  </tbody>
                </table>
              </div>
            </div>

            {/* A/B Comparison and Transport Player */}
            <div className="panel">
              <div className="panel-header">
                <span>Real-Time A/B Preview Player</span>
                <span className="status-badge active">DSP Engine Connected</span>
              </div>
              <div className="panel-body">
                {/* A/B Switch */}
                <div style={{ display: 'flex', gap: '8px', marginBottom: '14px' }}>
                  <button
                    className={`btn ${playerState.a_b_mode === 'original' ? 'btn-primary' : ''}`}
                    style={{ flex: 1 }}
                    onClick={() => handleABToggle('original')}
                  >
                    Original Sound (Bypass)
                  </button>
                  <button
                    className={`btn ${playerState.a_b_mode === 'processed' ? 'btn-primary' : ''}`}
                    style={{ flex: 1 }}
                    onClick={() => handleABToggle('processed')}
                  >
                    SonaTune Enhanced
                  </button>
                </div>

                <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', marginBottom: '12px' }}>
                  <label style={{ fontSize: '11.5px', display: 'flex', alignItems: 'center', gap: '6px', cursor: 'pointer' }}>
                    <input
                      type="checkbox"
                      checked={playerState.loudness_matched}
                      onChange={(e) => handleLoudnessMatchToggle(e.target.checked)}
                    />
                    <span>Loudness-Matched Comparison (Prevents volume bias)</span>
                  </label>
                  <span style={{ fontSize: '11px', color: 'var(--text-muted)' }}>
                    Mode: <strong>{playerState.a_b_mode.toUpperCase()}</strong>
                  </span>
                </div>

                {/* Scrubber */}
                <div style={{ marginBottom: '10px' }}>
                  <input
                    type="range"
                    className="slider-range"
                    min="0"
                    max={playerState.total_duration_seconds || 1}
                    step="0.1"
                    value={playerState.current_time_seconds}
                    onChange={(e) => handleSeek(parseFloat(e.target.value))}
                  />
                  <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: '11px', color: 'var(--text-muted)', marginTop: '4px' }}>
                    <span>{formatTime(playerState.current_time_seconds)}</span>
                    <span>{formatTime(playerState.total_duration_seconds)}</span>
                  </div>
                </div>

                {/* Play/Pause Button */}
                <div style={{ display: 'flex', gap: '8px' }}>
                  <button className="btn btn-primary" style={{ flex: 1 }} onClick={handlePlayPause}>
                    {playerState.is_playing ? 'Pause Playback' : 'Play Preview'}
                  </button>
                  <button className="btn" onClick={() => handleSeek(0)}>
                    Rewind
                  </button>
                </div>
              </div>
            </div>
          </div>

          {/* Export Settings Panel */}
          <div className="panel">
            <div className="panel-header">Batch Processing & Audio Export</div>
            <div className="panel-body">
              <div className="grid-3" style={{ marginBottom: '14px' }}>
                <div className="control-group">
                  <label className="control-label">DSP Enhancement Profile</label>
                  <select
                    className="select-input"
                    value={selectedPresetId}
                    onChange={(e) => setSelectedPresetId(e.target.value)}
                  >
                    {presets.map((p) => (
                      <option key={p.id} value={p.id}>
                        {p.name}
                      </option>
                    ))}
                  </select>
                </div>

                <div className="control-group">
                  <label className="control-label">Output Container Format</label>
                  <select
                    className="select-input"
                    value={exportFormat}
                    onChange={(e) => {
                      setExportFormat(e.target.value);
                      if (metadata) {
                        const baseName = metadata.file_name.substring(0, metadata.file_name.lastIndexOf('.')) || metadata.file_name;
                        setOutputFileName(`${baseName}_enhanced.${e.target.value}`);
                      }
                    }}
                  >
                    <option value="wav">WAV (16-bit PCM Lossless)</option>
                    <option value="flac">FLAC (Lossless Compressed)</option>
                    <option value="mp3">MP3 (High Quality 320 kbps)</option>
                  </select>
                </div>

                <div className="control-group">
                  <label className="control-label">Output File Name</label>
                  <input
                    type="text"
                    className="text-input"
                    value={outputFileName}
                    onChange={(e) => setOutputFileName(e.target.value)}
                  />
                </div>
              </div>

              <div className="control-group" style={{ marginBottom: '14px' }}>
                <label className="control-label">Destination Directory</label>
                <div style={{ display: 'flex', gap: '8px' }}>
                  <input
                    type="text"
                    className="text-input"
                    value={outputDir}
                    onChange={(e) => {
                      setOutputDir(e.target.value);
                      onUpdateExportDir?.(e.target.value);
                    }}
                  />
                  <button
                    className="btn btn-secondary"
                    onClick={handleBrowseOutputDir}
                    title="Select destination directory"
                  >
                    Browse...
                  </button>
                </div>
              </div>

              <div style={{ display: 'flex', gap: '10px', alignItems: 'center' }}>
                <button
                  className="btn btn-primary"
                  disabled={isProcessing}
                  onClick={handleExport}
                >
                  {isProcessing ? 'Processing Audio Stream...' : 'Export Enhanced Audio'}
                </button>

                {isProcessing && (
                  <button className="btn btn-danger" onClick={handleCancelExport}>
                    Cancel Export
                  </button>
                )}
              </div>

              {exportResult && (
                <div style={{ marginTop: '14px', padding: '10px 14px', backgroundColor: '#ecfdf5', border: '1px solid #34d399', borderRadius: '4px', color: '#065f46', fontSize: '12px' }}>
                  <div style={{ fontWeight: 600, marginBottom: '4px' }}>Export Complete and Verified!</div>
                  <div>File: <code>{exportResult.output_path}</code></div>
                  <div style={{ marginTop: '4px', color: '#047857' }}>
                    Original Peak: {exportResult.peak_original.toFixed(2)} (RMS {exportResult.rms_original.toFixed(2)}) • Processed Peak: {exportResult.peak_processed.toFixed(2)} (RMS {exportResult.rms_processed.toFixed(2)})
                  </div>
                </div>
              )}
            </div>
          </div>
        </>
      )}
    </div>
  );
};
