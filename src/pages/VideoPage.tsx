import React, { useState, useEffect } from 'react';
import { open } from '@tauri-apps/plugin-dialog';
import type { DspConfig, Preset, VideoMetadata, MediaToolInfo } from '../types';
import { apiInspectVideo, apiProcessVideo, apiCancelVideoProcessing } from '../api';

interface VideoPageProps {
  currentConfig: DspConfig;
  presets: Preset[];
  mediaTools: MediaToolInfo;
  lastExportDir?: string | null;
  onUpdateExportDir?: (dir: string) => void;
}

export const VideoPage: React.FC<VideoPageProps> = ({
  currentConfig,
  presets,
  mediaTools,
  lastExportDir,
  onUpdateExportDir,
}) => {
  const [videoPath, setVideoPath] = useState<string>('');
  const [metadata, setMetadata] = useState<VideoMetadata | null>(null);
  const [isLoading, setIsLoading] = useState<boolean>(false);
  const [errorMsg, setErrorMsg] = useState<string | null>(null);

  const [selectedAudioStream, setSelectedAudioStream] = useState<number>(0);
  const [selectedPresetId, setSelectedPresetId] = useState<string>('movies');
  const [outputFileName, setOutputFileName] = useState<string>('');
  const [outputDir, setOutputDir] = useState<string>(lastExportDir || '');
  const [isProcessing, setIsProcessing] = useState<boolean>(false);
  const [processSuccessMsg, setProcessSuccessMsg] = useState<string | null>(null);

  useEffect(() => {
    if (lastExportDir && !outputDir) {
      setOutputDir(lastExportDir);
    }
  }, [lastExportDir]);

  const handleInspect = async (path: string) => {
    if (!path.trim()) return;
    setIsLoading(true);
    setErrorMsg(null);
    setProcessSuccessMsg(null);

    // If output directory is not yet populated, derive it from input video location
    const fileFolder = path.substring(0, Math.max(path.lastIndexOf('\\'), path.lastIndexOf('/')));
    if (!outputDir && fileFolder) {
      setOutputDir(fileFolder);
      onUpdateExportDir?.(fileFolder);
    }

    try {
      const meta = await apiInspectVideo(path.trim());
      setMetadata(meta);

      if (meta.audio_streams.length > 0) {
        setSelectedAudioStream(meta.audio_streams[0].index);
      }

      const dotIdx = meta.file_name.lastIndexOf('.');
      const base = dotIdx !== -1 ? meta.file_name.substring(0, dotIdx) : meta.file_name;
      const ext = dotIdx !== -1 ? meta.file_name.substring(dotIdx) : '.mp4';
      setOutputFileName(`${base}_enhanced${ext}`);
    } catch (err: any) {
      setErrorMsg(typeof err === 'string' ? err : err.message || 'Failed to inspect video');
    } finally {
      setIsLoading(false);
    }
  };

  const handleBrowseVideo = async () => {
    try {
      const selected = await open({
        multiple: false,
        directory: false,
        filters: [
          {
            name: 'Video Files (*.mp4, *.mkv, *.mov, *.webm, *.avi, *.ts)',
            extensions: ['mp4', 'mkv', 'mov', 'webm', 'avi', 'ts'],
          },
          {
            name: 'All Files (*.*)',
            extensions: ['*'],
          },
        ],
      });
      if (selected && typeof selected === 'string') {
        setVideoPath(selected);
        handleInspect(selected);
      }
    } catch (err: any) {
      console.error('Video dialog error:', err);
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

  const handleExport = async () => {
    if (!metadata) return;
    const destPath = `${outputDir.replace(/\\$/, '')}\\${outputFileName}`;

    if (destPath === metadata.file_path) {
      setErrorMsg('Destination path cannot overwrite original video file.');
      return;
    }

    setIsProcessing(true);
    setErrorMsg(null);
    setProcessSuccessMsg(null);

    const activePreset = presets.find((p) => p.id === selectedPresetId);
    const cfg = activePreset ? activePreset.config : currentConfig;

    try {
      const result = await apiProcessVideo(metadata.file_path, destPath, selectedAudioStream, cfg);
      setProcessSuccessMsg(result);
    } catch (err: any) {
      setErrorMsg(typeof err === 'string' ? err : err.message || 'Video remuxing failed');
    } finally {
      setIsProcessing(false);
    }
  };

  const handleCancel = async () => {
    await apiCancelVideoProcessing();
    setIsProcessing(false);
  };

  const formatDuration = (secs: number) => {
    const m = Math.floor(secs / 60);
    const s = Math.floor(secs % 60);
    return `${m}m ${s}s`;
  };

  return (
    <div style={{ maxWidth: '960px', margin: '0 auto' }}>
      {/* FFmpeg Availability Notice */}
      {!mediaTools.ffmpeg_available && (
        <div style={{ padding: '10px 14px', marginBottom: '14px', backgroundColor: '#fffbeb', border: '1px solid #fde68a', borderRadius: '4px', fontSize: '12px', color: '#92400e' }}>
          <strong>Notice:</strong> FFmpeg was not detected in your Windows PATH. Video stream inspection and lossless stream-copy remuxing require FFmpeg. You can place <code>ffmpeg.exe</code> and <code>ffprobe.exe</code> in <code>C:\ffmpeg\bin</code> or your system PATH. Pure audio file processing remains fully functional via SonaTune's native Rust engine.
        </div>
      )}

      {/* Video File Picker */}
      <div className="panel">
        <div className="panel-header">Select Video File</div>
        <div className="panel-body">
          <div style={{ display: 'flex', gap: '8px' }}>
            <input
              type="text"
              className="text-input"
              placeholder="Enter absolute path to video file (MP4, MKV, MOV, WebM, AVI)..."
              value={videoPath}
              onChange={(e) => setVideoPath(e.target.value)}
              onKeyDown={(e) => e.key === 'Enter' && handleInspect(videoPath)}
            />
            <button
              className="btn btn-secondary"
              onClick={handleBrowseVideo}
              title="Browse computer for video file"
            >
              Browse...
            </button>
            <button
              className="btn btn-primary"
              disabled={isLoading || !videoPath.trim()}
              onClick={() => handleInspect(videoPath)}
            >
              {isLoading ? 'Inspecting...' : 'Analyze Video'}
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
          {/* Streams Overview */}
          <div className="grid-2">
            <div className="panel">
              <div className="panel-header">Video Streams (Preserved Losslessly)</div>
              <div className="panel-body" style={{ padding: 0 }}>
                <table className="data-table">
                  <thead>
                    <tr>
                      <th>Track</th>
                      <th>Codec</th>
                      <th>Resolution</th>
                      <th>FPS</th>
                    </tr>
                  </thead>
                  <tbody>
                    {metadata.video_streams.map((v) => (
                      <tr key={v.index}>
                        <td>Stream #{v.index}</td>
                        <td><code>{v.codec_name.toUpperCase()}</code></td>
                        <td>{v.width} × {v.height}</td>
                        <td>{v.frame_rate}</td>
                      </tr>
                    ))}
                    {metadata.video_streams.length === 0 && (
                      <tr>
                        <td colSpan={4} style={{ textAlign: 'center', color: 'var(--text-muted)' }}>
                          No video stream detected
                        </td>
                      </tr>
                    )}
                  </tbody>
                </table>
              </div>
            </div>

            <div className="panel">
              <div className="panel-header">Audio Streams to Enhance</div>
              <div className="panel-body" style={{ padding: 0 }}>
                <table className="data-table">
                  <thead>
                    <tr>
                      <th>Select</th>
                      <th>Track</th>
                      <th>Codec</th>
                      <th>Channels</th>
                      <th>Sample Rate</th>
                    </tr>
                  </thead>
                  <tbody>
                    {metadata.audio_streams.map((a) => (
                      <tr key={a.index}>
                        <td>
                          <input
                            type="radio"
                            name="audio_stream"
                            checked={selectedAudioStream === a.index}
                            onChange={() => setSelectedAudioStream(a.index)}
                          />
                        </td>
                        <td>Stream #{a.index} {a.language ? `(${a.language})` : ''}</td>
                        <td><code>{a.codec_name.toUpperCase()}</code></td>
                        <td>{a.channels === 1 ? 'Mono' : a.channels === 2 ? 'Stereo' : `${a.channels} Ch`}</td>
                        <td>{a.sample_rate} Hz</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            </div>
          </div>

          {/* Export Settings */}
          <div className="panel">
            <div className="panel-header">Lossless Remux & Video Export</div>
            <div className="panel-body">
              <div className="grid-3" style={{ marginBottom: '14px' }}>
                <div className="control-group">
                  <label className="control-label">Audio Enhancement Preset</label>
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
                  <label className="control-label">Video Strategy</label>
                  <input
                    type="text"
                    className="text-input"
                    value="Direct Stream Copy (-c:v copy, 0% quality loss)"
                    disabled
                  />
                </div>

                <div className="control-group">
                  <label className="control-label">Output Video Filename</label>
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
                  {isProcessing ? 'Processing & Remuxing...' : 'Export Enhanced Video'}
                </button>

                {isProcessing && (
                  <button className="btn btn-danger" onClick={handleCancel}>
                    Cancel Remuxing
                  </button>
                )}
              </div>

              {processSuccessMsg && (
                <div style={{ marginTop: '14px', padding: '10px 14px', backgroundColor: '#ecfdf5', border: '1px solid #34d399', borderRadius: '4px', color: '#065f46', fontSize: '12px' }}>
                  <strong>Success:</strong> {processSuccessMsg}
                </div>
              )}
            </div>
          </div>
        </>
      )}
    </div>
  );
};
