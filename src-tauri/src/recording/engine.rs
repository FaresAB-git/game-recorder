use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;
use std::time::{Duration, Instant};

use chrono::{DateTime, Utc};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, Command};
use tokio::sync::{oneshot, Mutex};
use uuid::Uuid;

use crate::config::AppConfig;
use crate::detection::GameType;
use crate::library::models::Replay;
use crate::recording::audio::{AudioCapture, AudioSource, AudioTrack};
use crate::recording::ffmpeg::{
    concat_segments, ffmpeg_cmd, find_ffmpeg, generate_thumbnail, get_duration, mux_av,
    remux_to_mp4, resolve_encoder, AudioInput, FfmpegError,
};

use windows::core::PCWSTR;
use windows::Win32::Foundation::POINT;
use windows::Win32::Graphics::Dxgi::{CreateDXGIFactory1, IDXGIFactory1, DXGI_OUTPUT_DESC};
use windows::Win32::Graphics::Gdi::{
    GetMonitorInfoW, MonitorFromPoint, MonitorFromWindow, MONITORINFOEXW, MONITOR_DEFAULTTOPRIMARY,
};
use windows::Win32::UI::WindowsAndMessaging::FindWindowW;

#[derive(Debug, thiserror::Error)]
pub enum EngineError {
    #[error("FFmpeg error: {0}")]
    Ffmpeg(#[from] FfmpegError),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Already recording")]
    AlreadyRecording,
    #[error("FFmpeg not found")]
    FfmpegNotFound,
    #[error("{0}")]
    Other(String),
}

/// Everything needed to (re)spawn the video capture.
#[derive(Clone)]
struct VideoParams {
    ffmpeg: PathBuf,
    fps: u32,
    bitrate_kbps: u32,
    enc: String,
    resolution: String,
    game: GameType,
    output_dir: PathBuf,
    stem: String,
    /// Use the CPU filter path (hwdownload) instead of zero-copy GPU frames.
    /// Turned on automatically when the GPU path fails.
    cpu_filter: bool,
}

/// One continuous piece of video. A new one is created each time FFmpeg has to
/// be restarted (alt-tab out of exclusive fullscreen kills ddagrab).
#[derive(Clone)]
struct Segment {
    path: PathBuf,
    spawned: Instant,
    /// When capture of this segment stopped (FFmpeg exit or 'q' sent).
    ended: Option<Instant>,
}

type Segments = Arc<std::sync::Mutex<Vec<Segment>>>;

struct ActiveRecording {
    game: GameType,
    started_at: DateTime<Utc>,
    id: String,
    stem: String,
    output_dir: PathBuf,
    segments: Segments,
    audio: Vec<AudioCapture>,
    stop_tx: Option<oneshot::Sender<()>>,
    watcher: Option<tokio::task::JoinHandle<()>>,
}

pub struct RecordingEngine {
    active: Mutex<Option<ActiveRecording>>,
    /// Result of encoder auto-detection, computed once at startup.
    cached_encoder: Mutex<Option<String>>,
}

impl RecordingEngine {
    pub fn new() -> Arc<Self> {
        Arc::new(Self { active: Mutex::new(None), cached_encoder: Mutex::new(None) })
    }

    /// Probe encoders at app startup so no test process runs during a game.
    pub async fn warmup_encoder(&self) {
        if let Some(ffmpeg) = find_ffmpeg() {
            let enc = resolve_encoder(&ffmpeg, "auto").await;
            log::info!("Encoder pre-detected at startup: {}", enc);
            *self.cached_encoder.lock().await = Some(enc);
        } else {
            log::error!("FFmpeg not found — recording will not work");
        }
    }

    async fn pick_encoder(&self, ffmpeg: &Path, requested: &str) -> String {
        if requested != "auto" {
            return resolve_encoder(ffmpeg, requested).await;
        }
        let mut cache = self.cached_encoder.lock().await;
        if let Some(enc) = cache.as_ref() {
            return enc.clone();
        }
        let enc = resolve_encoder(ffmpeg, "auto").await;
        *cache = Some(enc.clone());
        enc
    }

    pub async fn is_recording(&self) -> bool {
        self.active.lock().await.is_some()
    }

    pub async fn start_recording(&self, game: GameType, config: &AppConfig) -> Result<(), EngineError> {
        let mut guard = self.active.lock().await;
        if guard.is_some() {
            return Err(EngineError::AlreadyRecording);
        }

        let ffmpeg = find_ffmpeg().ok_or(EngineError::FfmpegNotFound)?;
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();
        let output_dir = PathBuf::from(&config.output_dir);
        std::fs::create_dir_all(&output_dir)?;
        let stem = format!("{}_{}", game.tag(), now.format("%Y-%m-%d_%H-%M-%S"));

        let enc = self.pick_encoder(&ffmpeg, &config.encoder).await;
        let mut params = VideoParams {
            ffmpeg,
            fps: config.fps.clamp(15, 240),
            bitrate_kbps: config.bitrate.clamp(1000, 150_000),
            enc,
            resolution: config.resolution.clone(),
            game,
            output_dir: output_dir.clone(),
            stem: stem.clone(),
            cpu_filter: false,
        };

        // Audio first: it must already be running when the first video frame
        // is captured. The exact offset is computed at the end anyway.
        let mut audio = Vec::new();
        if config.capture_game_audio {
            match AudioCapture::start(AudioSource::Loopback, output_dir.join(format!("{}_game.wav", stem))) {
                Ok(a) => audio.push(a),
                Err(e) => log::error!("Game audio capture failed to start: {}", e),
            }
        }
        if config.capture_mic {
            match AudioCapture::start(AudioSource::Mic(config.mic_device.clone()), output_dir.join(format!("{}_mic.wav", stem))) {
                Ok(a) => audio.push(a),
                Err(e) => log::error!("Mic capture failed to start: {}", e),
            }
        }

        // ddagrab can fail with DXGI_ERROR_ACCESS_LOST while the game switches
        // display mode (loading screen → fullscreen), so retry a few times.
        let first_path = output_dir.join(format!("{}.mkv", stem));
        let mut started: Option<(Child, Option<ChildStdin>, Instant)> = None;
        for attempt in 1..=5 {
            // After 2 failures, fall back to the CPU filter path (works everywhere).
            params.cpu_filter = attempt > 2;
            match spawn_video(&params, &first_path).await {
                Ok(s) => {
                    started = Some(s);
                    break;
                }
                Err(e) => {
                    log::warn!("Video start attempt {}/5 failed: {}", attempt, e);
                    let _ = std::fs::remove_file(&first_path);
                    tokio::time::sleep(Duration::from_secs(3)).await;
                }
            }
        }
        let Some((child, stdin, spawned)) = started else {
            for a in audio {
                if let Some(t) = a.stop() {
                    let _ = std::fs::remove_file(&t.path);
                }
            }
            return Err(EngineError::Other("FFmpeg failed to start after 5 attempts".into()));
        };

        log::info!("Recording started: {} enc={} → {}", game.tag(), params.enc, first_path.display());

        let segments: Segments = Arc::new(std::sync::Mutex::new(vec![Segment {
            path: first_path,
            spawned,
            ended: None,
        }]));
        let (stop_tx, stop_rx) = oneshot::channel();
        let watcher = tokio::spawn(run_watcher(params, child, stdin, Arc::clone(&segments), stop_rx));

        *guard = Some(ActiveRecording {
            game,
            started_at: now,
            id,
            stem,
            output_dir,
            segments,
            audio,
            stop_tx: Some(stop_tx),
            watcher: Some(watcher),
        });
        Ok(())
    }

    /// Stop capture and build the final MP4. Returns `None` if nothing usable
    /// was recorded (or the game was shorter than the configured minimum).
    pub async fn stop_recording(&self, config: &AppConfig) -> Result<Option<Replay>, EngineError> {
        let mut rec = match self.active.lock().await.take() {
            Some(r) => r,
            None => return Ok(None),
        };

        // Stop video first so its end instant is as close as possible to the
        // audio's; audio keeps rolling a few ms longer, which is harmless.
        if let Some(tx) = rec.stop_tx.take() {
            let _ = tx.send(());
        }
        if let Some(handle) = rec.watcher.take() {
            // restart window (3 s) + health check (2 s) + graceful stop (20 s)
            if tokio::time::timeout(Duration::from_secs(30), handle).await.is_err() {
                log::warn!("Watcher did not finish in time");
            }
        }
        let tracks: Vec<AudioTrack> = rec.audio.drain(..).filter_map(|a| a.stop()).collect();

        let segments: Vec<Segment> = rec.segments.lock().map(|g| g.clone()).unwrap_or_default();
        let all_files: Vec<PathBuf> = segments
            .iter()
            .map(|s| s.path.clone())
            .chain(tracks.iter().map(|t| t.path.clone()))
            .collect();
        let cleanup = |extra: &[&Path]| {
            for f in &all_files {
                let _ = std::fs::remove_file(f);
            }
            for f in extra {
                let _ = std::fs::remove_file(f);
            }
        };

        let elapsed = (Utc::now() - rec.started_at).num_seconds();
        let min_secs = config.min_game_duration_minutes as i64 * 60;
        if elapsed < min_secs {
            log::info!("Game too short ({} s < {} s) — discarding", elapsed, min_secs);
            cleanup(&[]);
            return Ok(None);
        }

        // Keep segments that actually contain video, with their real duration.
        let mut valid: Vec<(Segment, f64)> = Vec::new();
        for seg in segments {
            let size = std::fs::metadata(&seg.path).map(|m| m.len()).unwrap_or(0);
            if size < 16 * 1024 {
                continue;
            }
            let ended = seg.ended.unwrap_or_else(Instant::now);
            let dur = match get_duration(&seg.path).await {
                Some(d) => d,
                None => ended.saturating_duration_since(seg.spawned).as_secs_f64() - 0.5,
            };
            if dur > 0.5 {
                valid.push((seg, dur));
            }
        }
        log::info!("Finalizing: {} usable segment(s), {} audio track(s)", valid.len(), tracks.len());

        if valid.is_empty() {
            log::error!("No usable video segment — discarding");
            cleanup(&[]);
            return Ok(None);
        }

        let ffmpeg = find_ffmpeg().ok_or(EngineError::FfmpegNotFound)?;
        let out_dir = rec.output_dir.clone();
        let mp4 = out_dir.join(format!("{}_{}.mp4", rec.stem, &rec.id[..8]));

        let concat_path = out_dir.join(format!("{}_concat.mkv", rec.stem));
        let video_src: PathBuf = if valid.len() == 1 {
            valid[0].0.path.clone()
        } else {
            let paths: Vec<PathBuf> = valid.iter().map(|(s, _)| s.path.clone()).collect();
            match concat_segments(&ffmpeg, &paths, &concat_path).await {
                Ok(()) => concat_path.clone(),
                Err(e) => {
                    log::warn!("Concat failed ({}), keeping first segment only", e);
                    valid.truncate(1);
                    valid[0].0.path.clone()
                }
            }
        };

        // Each segment's start on the wall clock = its end minus its length.
        // End instants are precise (we send 'q' / notice the crash within
        // 100 ms), unlike start, which depends on FFmpeg's init time.
        let durations: Vec<f64> = valid.iter().map(|(_, d)| *d).collect();
        let seg_starts: Vec<Instant> = valid
            .iter()
            .map(|(s, d)| {
                let end = s.ended.unwrap_or_else(Instant::now);
                end.checked_sub(Duration::from_secs_f64(*d)).unwrap_or(s.spawned)
            })
            .collect();

        let inputs: Vec<AudioInput> = tracks
            .iter()
            .filter(|t| std::fs::metadata(&t.path).map(|m| m.len() > 44).unwrap_or(false))
            .map(|t| AudioInput {
                path: &t.path,
                offsets: seg_starts.iter().map(|s| signed_secs(*s, t.start)).collect(),
            })
            .collect();
        for t in &inputs {
            log::info!("Audio {} offsets (s): {:?}", t.path.display(), t.offsets);
        }

        let mut ok = false;
        if !inputs.is_empty() {
            match mux_av(&ffmpeg, &video_src, &durations, &inputs, &mp4).await {
                Ok(()) => ok = true,
                Err(e) => log::warn!("Audio mux failed ({}), saving video only", e),
            }
        }
        if !ok {
            if let Err(e) = remux_to_mp4(&ffmpeg, &video_src, &mp4).await {
                // Last resort: keep the MKV so the game isn't lost.
                log::error!("Remux failed ({}) — keeping the MKV", e);
                let keep = out_dir.join(format!("{}_{}.mkv", rec.stem, &rec.id[..8]));
                let _ = std::fs::rename(&video_src, &keep);
                cleanup(&[&concat_path]);
                return Ok(Some(self.make_replay(&rec, &keep, elapsed as f64, None)));
            }
        }
        cleanup(&[&concat_path]);

        let duration = get_duration(&mp4).await.unwrap_or(elapsed as f64);
        let thumb = mp4.with_extension("jpg");
        let thumb = generate_thumbnail(&ffmpeg, &mp4, &thumb, duration).await.ok().map(|_| thumb);
        Ok(Some(self.make_replay(&rec, &mp4, duration, thumb)))
    }

    fn make_replay(&self, rec: &ActiveRecording, path: &Path, duration: f64, thumb: Option<PathBuf>) -> Replay {
        Replay {
            id: rec.id.clone(),
            game: rec.game.as_str().to_string(),
            date: rec.started_at.to_rfc3339(),
            duration: duration.round() as i64,
            size: std::fs::metadata(path).map(|m| m.len() as i64).unwrap_or(0),
            path: path.to_string_lossy().to_string(),
            thumbnail_path: thumb.map(|t| t.to_string_lossy().to_string()),
            favorite: false,
            is_new: true,
            is_clip: false,
            source_id: None,
        }
    }
}

/// `a - b` in seconds, can be negative.
fn signed_secs(a: Instant, b: Instant) -> f64 {
    if a >= b {
        (a - b).as_secs_f64()
    } else {
        -(b - a).as_secs_f64()
    }
}

// ── watcher ───────────────────────────────────────────────────────────────────

/// Keeps FFmpeg alive for the whole game: if it dies (alt-tab in exclusive
/// fullscreen), a new segment is started. Stops gracefully on `stop_rx`.
async fn run_watcher(
    mut params: VideoParams,
    mut process: Child,
    mut stdin: Option<ChildStdin>,
    segments: Segments,
    mut stop_rx: oneshot::Receiver<()>,
) {
    let mark_ended = |segments: &Segments| {
        if let Ok(mut g) = segments.lock() {
            if let Some(last) = g.last_mut() {
                last.ended.get_or_insert_with(Instant::now);
            }
        }
    };
    let mut seg_counter = 2u32;

    loop {
        // Wait for either a stop request or FFmpeg exiting.
        let died = tokio::select! {
            _ = &mut stop_rx => false,
            status = process.wait() => {
                mark_ended(&segments);
                log::warn!("FFmpeg exited mid-recording ({:?}) — restarting in 3 s", status.ok());
                true
            }
        };

        if !died {
            mark_ended(&segments);
            graceful_stop(&mut process, stdin.take()).await;
            return;
        }

        // Restart after a short pause (the game is usually switching modes).
        tokio::select! {
            _ = &mut stop_rx => return,
            _ = tokio::time::sleep(Duration::from_secs(3)) => {}
        }

        let path = params.output_dir.join(format!("{}_seg{}.mkv", params.stem, seg_counter));
        seg_counter += 1;
        match spawn_video(&params, &path).await {
            Ok((p, s, spawned)) => {
                log::info!("Recording resumed → {}", path.display());
                if let Ok(mut g) = segments.lock() {
                    g.push(Segment { path, spawned, ended: None });
                }
                process = p;
                stdin = s;
            }
            Err(e) => {
                // Keep trying until the game ends: alt-tab can last a while.
                log::warn!("Restart failed: {} — will retry", e);
                params.cpu_filter = true;
                let _ = std::fs::remove_file(&path);
                tokio::select! {
                    _ = &mut stop_rx => return,
                    _ = tokio::time::sleep(Duration::from_secs(2)) => {}
                }
                // Loop again with a dummy "dead" process: spawn_video failed,
                // so wait() on the old one returns immediately.
            }
        }
    }
}

/// Ask FFmpeg to finish the file ('q'), kill it if it doesn't within 20 s.
async fn graceful_stop(process: &mut Child, stdin: Option<ChildStdin>) {
    if let Some(mut s) = stdin {
        let _ = s.write_all(b"q").await;
        let _ = s.flush().await;
    }
    if tokio::time::timeout(Duration::from_secs(20), process.wait()).await.is_err() {
        log::warn!("FFmpeg did not stop in time — killing it");
        let _ = process.kill().await;
    }
}

/// Spawn FFmpeg and make sure it is still alive after 2 s.
async fn spawn_video(params: &VideoParams, output: &Path) -> Result<(Child, Option<ChildStdin>, Instant), EngineError> {
    let (idx, mw, mh) = find_game_output(params.game);
    let mut cmd = build_video_command(params, output, idx, mw, mh);
    let spawned = Instant::now();
    let mut child = cmd.spawn()?;
    let stdin = child.stdin.take();

    if let Some(stderr) = child.stderr.take() {
        tokio::spawn(async move {
            let mut lines = BufReader::new(stderr).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                let line = line.trim();
                if !line.is_empty() {
                    log::warn!("FFmpeg: {}", line);
                }
            }
        });
    }

    tokio::time::sleep(Duration::from_secs(2)).await;
    if let Some(status) = child.try_wait()? {
        return Err(EngineError::Other(format!("FFmpeg exited immediately ({})", status)));
    }
    Ok((child, stdin, spawned))
}

/// Find the DXGI output index and resolution of the monitor the game is on.
/// Falls back to (0, 1920, 1080).
fn find_game_output(game: GameType) -> (u32, u32, u32) {
    const FALLBACK: (u32, u32, u32) = (0, 1920, 1080);
    let titles: &[&str] = match game {
        GameType::Lol => &["League of Legends (TM) Client", "League of Legends"],
        // The Valorant window title has two trailing spaces.
        GameType::Valorant => &["VALORANT  ", "VALORANT"],
    };

    unsafe {
        let hwnd = titles.iter().find_map(|title| {
            let wide: Vec<u16> = title.encode_utf16().chain(std::iter::once(0)).collect();
            let h = FindWindowW(PCWSTR::null(), PCWSTR(wide.as_ptr()));
            (h.0 as isize != 0).then_some(h)
        });
        let hmonitor = match hwnd {
            Some(h) => MonitorFromWindow(h, MONITOR_DEFAULTTOPRIMARY),
            None => {
                log::warn!("Game window not found — using primary monitor");
                MonitorFromPoint(POINT::default(), MONITOR_DEFAULTTOPRIMARY)
            }
        };

        let mut mi = std::mem::zeroed::<MONITORINFOEXW>();
        mi.monitorInfo.cbSize = std::mem::size_of::<MONITORINFOEXW>() as u32;
        if !GetMonitorInfoW(hmonitor, &mut mi.monitorInfo).as_bool() {
            return FALLBACK;
        }
        let monitor_dev = String::from_utf16_lossy(&mi.szDevice).trim_end_matches('\0').to_string();

        let Ok(factory) = CreateDXGIFactory1::<IDXGIFactory1>() else { return FALLBACK };
        // ddagrab enumerates the outputs of the default adapter.
        let Ok(adapter) = factory.EnumAdapters1(0) else { return FALLBACK };

        let mut i = 0u32;
        while let Ok(output) = adapter.EnumOutputs(i) {
            let mut desc = std::mem::zeroed::<DXGI_OUTPUT_DESC>();
            if output.GetDesc(&mut desc).is_ok() {
                let dev = String::from_utf16_lossy(&desc.DeviceName).trim_end_matches('\0').to_string();
                if dev == monitor_dev {
                    let r = desc.DesktopCoordinates;
                    let w = ((r.right - r.left) as u32) & !1;
                    let h = ((r.bottom - r.top) as u32) & !1;
                    log::info!("Game monitor {} → output_idx={} {}x{}", dev, i, w, h);
                    return (i, w, h);
                }
            }
            i += 1;
        }
        log::warn!("No DXGI output matched '{}' — using output 0", monitor_dev);
        FALLBACK
    }
}

/// FFmpeg ddagrab → H.264 MKV (video only, audio is captured separately).
fn build_video_command(p: &VideoParams, output: &Path, output_idx: u32, monitor_w: u32, monitor_h: u32) -> Command {
    let hw_enc = matches!(p.enc.as_str(), "h264_nvenc" | "h264_amf");
    let (w, h) = match p.resolution.as_str() {
        "1080p" if monitor_h > 1080 => (1920, 1080),
        "720p" if monitor_h > 720 => (1280, 720),
        _ => (monitor_w, monitor_h),
    };
    let (w, h) = (w & !1, h & !1);
    let fps = p.fps;

    // Zero-copy (frames stay on the GPU) only at native resolution:
    // scale_d3d11 is broken in current FFmpeg builds, so resizing goes through the CPU.
    let hw = hw_enc && !p.cpu_filter && (w, h) == (monitor_w & !1, monitor_h & !1);
    let filter = if hw {
        format!("ddagrab=framerate={fps}:output_idx={output_idx}[v]")
    } else {
        format!("ddagrab=framerate={fps}:output_idx={output_idx},hwdownload,format=bgra,scale={w}:{h},format=nv12[v]")
    };

    let br = p.bitrate_kbps;
    let (b, maxrate, bufsize) = (format!("{br}k"), format!("{}k", br * 3 / 2), format!("{}k", br * 2));
    let gop = fps.to_string(); // 1 keyframe per second → precise clip cuts

    let mut cmd = ffmpeg_cmd(&p.ffmpeg);
    cmd.args(["-hide_banner", "-nostats", "-loglevel", "warning"]);
    if hw {
        cmd.args(["-init_hw_device", "d3d11va=d3d"]);
    }
    cmd.args(["-filter_complex", &filter, "-map", "[v]", "-c:v", &p.enc]);
    match p.enc.as_str() {
        "h264_nvenc" => cmd.args(["-preset", "p4", "-tune", "hq", "-rc", "vbr"]),
        "h264_amf" => cmd.args(["-quality", "balanced", "-rc", "vbr_peak"]),
        "h264_qsv" => cmd.args(["-preset", "faster"]),
        _ => cmd.args(["-preset", "veryfast"]),
    };
    cmd.args(["-b:v", &b, "-maxrate", &maxrate, "-bufsize", &bufsize, "-g", &gop]);
    cmd.args(["-an", "-f", "matroska", "-y"]).arg(output);
    cmd.stdin(Stdio::piped()).stdout(Stdio::null()).stderr(Stdio::piped());
    log::info!(
        "ddagrab output_idx={} {}x{} @ {} fps, {} @ {} kbps ({})",
        output_idx, w, h, fps, p.enc, br, if hw { "GPU zero-copy" } else { "CPU filter" }
    );
    cmd
}
