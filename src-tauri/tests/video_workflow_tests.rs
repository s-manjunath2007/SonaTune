use sonatune::dsp::DspConfig;
use sonatune::media::{detect_ffmpeg, inspect_video_file, process_and_remux_video};
use std::process::Command;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

#[test]
fn test_ffmpeg_and_ffprobe_detection() {
    let tools = detect_ffmpeg();
    println!("FFmpeg available: {}", tools.ffmpeg_available);
    println!("FFprobe available: {}", tools.ffprobe_available);
    println!("FFmpeg path: {:?}", tools.ffmpeg_path);
    println!("Version: {:?}", tools.version);

    assert!(tools.ffmpeg_available, "FFmpeg should be detected on the system");
    assert!(tools.ffprobe_available, "FFprobe should be detected on the system");
}

#[test]
fn test_video_audio_remux_workflow_end_to_end() {
    let tools = detect_ffmpeg();
    if !tools.ffmpeg_available || !tools.ffprobe_available {
        println!("Skipping end-to-end video test: ffmpeg/ffprobe not in path");
        return;
    }

    let ffmpeg_cmd = tools.ffmpeg_path.unwrap();
    let temp_dir = std::env::temp_dir();
    let in_video = temp_dir.join("sonatune_test_synth_in.mp4");
    let out_video = temp_dir.join("sonatune_test_synth_out.mp4");

    let in_str = in_video.to_str().unwrap();
    let out_str = out_video.to_str().unwrap();

    let _ = std::fs::remove_file(&in_video);
    let _ = std::fs::remove_file(&out_video);

    // 1. Generate 1.0s synthetic video with video testsrc and audio sine wave
    let gen_status = Command::new(&ffmpeg_cmd)
        .args([
            "-y",
            "-f", "lavfi",
            "-i", "testsrc=duration=1:size=320x240:rate=30",
            "-f", "lavfi",
            "-i", "sine=frequency=440:duration=1",
            "-c:v", "libx264",
            "-pix_fmt", "yuv420p",
            "-c:a", "aac",
            "-b:a", "128k",
            in_str,
        ])
        .status()
        .expect("Failed to invoke ffmpeg to generate test video");

    assert!(gen_status.success(), "Failed to generate test input video");
    assert!(in_video.exists(), "Input test video was not created");
    let orig_file_size = std::fs::metadata(&in_video).unwrap().len();

    // 2. Inspect with SonaTune media engine
    let meta = inspect_video_file(in_str).expect("Failed to inspect test video with SonaTune");
    assert!(!meta.video_streams.is_empty(), "Must detect at least 1 video stream");
    assert!(!meta.audio_streams.is_empty(), "Must detect at least 1 audio stream");
    assert_eq!(meta.video_streams[0].width, 320);
    assert_eq!(meta.video_streams[0].height, 240);
    assert!(meta.duration_seconds >= 0.8 && meta.duration_seconds <= 1.2);

    let audio_stream_idx = meta.audio_streams[0].index;

    // 3. Apply SonaTune DSP processing and remux (video stream copy)
    let mut config = DspConfig::default();
    config.bass_gain_db = 6.0;
    config.vocal_clarity_db = 4.0;
    config.limiter_enabled = true;

    let cancel_flag = Arc::new(AtomicBool::new(false));
    let res = process_and_remux_video(in_str, out_str, audio_stream_idx, &config, cancel_flag);
    assert!(res.is_ok(), "Video processing failed: {:?}", res.err());

    // 4. Verify output file exists and input was untouched
    assert!(out_video.exists(), "Exported video file does not exist");
    let out_file_size = std::fs::metadata(&out_video).unwrap().len();
    assert!(out_file_size > 1000, "Exported video is too small / empty");
    assert_eq!(std::fs::metadata(&in_video).unwrap().len(), orig_file_size, "Source file must not be modified");

    // 5. Inspect exported video and confirm video stream preserved and audio stream present
    let out_meta = inspect_video_file(out_str).expect("Failed to inspect exported video");
    assert!(!out_meta.video_streams.is_empty(), "Exported video must contain video stream");
    assert_eq!(out_meta.video_streams[0].width, 320);
    assert_eq!(out_meta.video_streams[0].height, 240);
    assert!(!out_meta.audio_streams.is_empty(), "Exported video must contain audio stream");
    assert!(out_meta.duration_seconds >= 0.8 && out_meta.duration_seconds <= 1.2, "Duration must remain synchronized");

    // Clean up
    let _ = std::fs::remove_file(in_video);
    let _ = std::fs::remove_file(out_video);
}
