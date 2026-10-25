use std::path::{Path, PathBuf};
use std::process::Stdio;
use thiserror::Error;
use tokio::process::Command;

/// Spawn an FFmpeg/FFprobe command without any console window on Windows.
pub fn ffmpeg_cmd(bin: &Path) -> Command {
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        let mut std_cmd = std::process::Command::new(bin);
        // CREATE_NO_WINDOW (0x08000000): never allocate a console. Without it a
        // console pops up and steals focus from the game (the "auto alt-tab" bug).
        std_cmd.creation_flags(0x0800_0000);
        let mut cmd = Command::from(std_cmd);
        cmd.kill_on_drop(true);
        cmd
    }
    #[cfg(not(target_os = "windows"))]
    {
        let mut cmd = Command::new(bin);
        cmd.kill_on_drop(true);
        cmd
    }
}

#[derive(Debug, Error)]
pub enum FfmpegError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("FFmpeg error: {0}")]
    Process(String),
}

fn path_str(p: &Path) -> &str {
    p.to_str().unwrap_or_default()
}

/// Find a bundled binary (`ffmpeg` / `ffprobe`):
/// next to the exe (installed), then `src-tauri/binaries` (dev), then PATH.
fn find_binary(name: &str) -> Option<PathBuf> {
    let triple = format!("{}-x86_64-pc-windows-msvc.exe", name);
    let plain = format!("{}.exe", name);

    if let Ok(exe) = std::env::current_exe() {
        let dir = exe.parent().unwrap_or(Path::new("."));
        for n in [&plain, &triple] {
            let p = dir.join(n);
            if p.exists() {
                return Some(p);
            }
        }
        // Dev: target/{debug,release}/game-recorder.exe → ../../binaries/
        if let Ok(p) = dir.join("..").join("..").join("binaries").join(&triple).canonicalize() {
            return Some(p);
        }
    }

    if name == "ffmpeg" {
        if let Ok(p) = std::env::var("FFMPEG_PATH") {
            let p = PathBuf::from(p);
            if p.exists() {
                return Some(p);
            }
        }
    }

    std::env::var_os("PATH").and_then(|paths| {
        std::env::split_paths(&paths)
            .map(|d| d.join(&plain))
            .find(|p| p.exists())
    })
}

pub fn find_ffmpeg() -> Option<PathBuf> {
    find_binary("ffmpeg")
}

fn find_ffprobe() -> Option<PathBuf> {
    find_binary("ffprobe")
}

async fn run(mut cmd: Command, what: &str) -> Result<(), FfmpegError> {
    let out = cmd.stdout(Stdio::null()).stderr(Stdio::piped()).output().await?;
    if out.status.success() {
        return Ok(());
    }
    let stderr = String::from_utf8_lossy(&out.stderr);
    let tail: Vec<&str> = stderr.lines().rev().take(6).collect();
    let tail = tail.into_iter().rev().collect::<Vec<_>>().join(" | ");
    log::error!("{} failed ({:?}): {}", what, out.status.code(), tail);
    Err(FfmpegError::Process(format!("{} failed ({:?})", what, out.status.code())))
}

/// Remux to MP4 without re-encoding.
pub async fn remux_to_mp4(ffmpeg: &Path, input: &Path, mp4: &Path) -> Result<(), FfmpegError> {
    let mut cmd = ffmpeg_cmd(ffmpeg);
    cmd.args(["-hide_banner", "-i", path_str(input), "-map", "0", "-c", "copy", "-movflags", "+faststart", "-y", path_str(mp4)]);
    run(cmd, "remux").await
}

/// Concatenate MKV segments (stream copy) with the concat demuxer.
pub async fn concat_segments(ffmpeg: &Path, segs: &[PathBuf], output: &Path) -> Result<(), FfmpegError> {
    let list_path = output.with_extension("concat.txt");
    let content: String = segs
        .iter()
        .map(|s| format!("file '{}'\n", s.to_string_lossy().replace('\\', "/").replace('\'', "'\\''")))
        .collect();
    std::fs::write(&list_path, content)?;

    let mut cmd = ffmpeg_cmd(ffmpeg);
    cmd.args(["-hide_banner", "-f", "concat", "-safe", "0", "-i", path_str(&list_path), "-c", "copy", "-y", path_str(output)]);
    let res = run(cmd, "concat").await;
    let _ = std::fs::remove_file(&list_path);
    res
}

/// One audio track to lay under the video.
pub struct AudioInput<'a> {
    pub path: &'a Path,
    /// For each video segment: where that segment starts in this WAV, in
    /// seconds (negative = the segment started before the audio did).
    pub offsets: Vec<f64>,
}

/// Mux the (concatenated) video with one or more WAV tracks into an MP4.
///
/// `seg_durations` are the durations of the video segments in order. Each
/// audio track is cut into the matching slices so the sound follows the video
/// exactly, even when segments were split by an alt-tab restart.
pub async fn mux_av(
    ffmpeg: &Path,
    video: &Path,
    seg_durations: &[f64],
    audio: &[AudioInput<'_>],
    mp4: &Path,
) -> Result<(), FfmpegError> {
    let n = seg_durations.len();
    let mut graph: Vec<String> = Vec::new();
    let mut track_labels: Vec<String> = Vec::new();

    for (k, track) in audio.iter().enumerate() {
        let input = format!("[{}:a]", k + 1);
        let sources: Vec<String> = if n == 1 {
            vec![input.clone()]
        } else {
            let outs: Vec<String> = (0..n).map(|i| format!("[t{}s{}]", k, i)).collect();
            graph.push(format!("{}asplit={}{}", input, n, outs.join("")));
            outs
        };

        let mut pieces = Vec::new();
        for (i, (&dur, src)) in seg_durations.iter().zip(&sources).enumerate() {
            let off = track.offsets.get(i).copied().unwrap_or(0.0);
            let start = off.max(0.0);
            let end = (off + dur).max(start + 0.001);
            let delay_ms = ((-off).max(0.0) * 1000.0).round() as i64;
            let label = format!("[t{}p{}]", k, i);
            let mut chain = format!("{}atrim=start={:.4}:end={:.4},asetpts=PTS-STARTPTS", src, start, end);
            if delay_ms > 0 {
                chain.push_str(&format!(",adelay=delays={}:all=1", delay_ms));
            }
            // Force the slice to the exact segment length (pads or cuts).
            chain.push_str(&format!(",apad=whole_dur={:.4},atrim=end={:.4}{}", dur, dur, label));
            graph.push(chain);
            pieces.push(label);
        }

        let track_label = format!("[t{}]", k);
        if n == 1 {
            // Rename the single piece.
            let last = graph.pop().unwrap();
            graph.push(last.replace(&pieces[0], &track_label));
        } else {
            graph.push(format!("{}concat=n={}:v=0:a=1{}", pieces.join(""), n, track_label));
        }
        track_labels.push(track_label);
    }

    let out_label = if track_labels.len() == 1 {
        track_labels[0].clone()
    } else {
        graph.push(format!(
            "{}amix=inputs={}:duration=longest:normalize=0[aout]",
            track_labels.join(""),
            track_labels.len()
        ));
        "[aout]".to_string()
    };

    let filter = graph.join(";");
    log::info!("Audio filter: {}", filter);

    let mut cmd = ffmpeg_cmd(ffmpeg);
    cmd.args(["-hide_banner", "-i", path_str(video)]);
    for t in audio {
        cmd.args(["-i", path_str(t.path)]);
    }
    cmd.args([
        "-filter_complex", &filter,
        "-map", "0:v:0", "-map", &out_label,
        "-c:v", "copy",
        "-c:a", "aac", "-b:a", "192k", "-ac", "2",
        "-movflags", "+faststart",
        "-y", path_str(mp4),
    ]);
    run(cmd, "mux").await
}

/// Generate a thumbnail at ~30% of the duration.
pub async fn generate_thumbnail(ffmpeg: &Path, video: &Path, thumb: &Path, duration_secs: f64) -> Result<(), FfmpegError> {
    let seek = if duration_secs > 10.0 { (duration_secs * 0.30).max(5.0) } else { duration_secs / 2.0 };
    let mut cmd = ffmpeg_cmd(ffmpeg);
    cmd.args([
        "-hide_banner", "-ss", &format!("{:.2}", seek), "-i", path_str(video),
        "-frames:v", "1", "-vf", "scale=640:-2", "-q:v", "3", "-y", path_str(thumb),
    ]);
    run(cmd, "thumbnail").await
}

/// Duration of a media file in seconds (ffprobe). `None` if unknown.
pub async fn get_duration(video: &Path) -> Option<f64> {
    let ffprobe = find_ffprobe()?;
    let output = ffmpeg_cmd(&ffprobe)
        .args(["-v", "error", "-show_entries", "format=duration", "-of", "default=noprint_wrappers=1:nokey=1", path_str(video)])
        .stderr(Stdio::null())
        .output()
        .await
        .ok()?;
    String::from_utf8_lossy(&output.stdout).trim().parse::<f64>().ok().filter(|d| d.is_finite() && *d > 0.0)
}

/// Map the user's encoder setting to an FFmpeg encoder name ("auto" probes the GPU).
pub async fn resolve_encoder(ffmpeg: &Path, requested: &str) -> String {
    match requested {
        "nvenc" => "h264_nvenc".to_string(),
        "amf" => "h264_amf".to_string(),
        "qsv" => "h264_qsv".to_string(),
        "x264" => "libx264".to_string(),
        _ => {
            for enc in ["h264_nvenc", "h264_amf", "h264_qsv"] {
                if test_encoder(ffmpeg, enc).await {
                    return enc.to_string();
                }
            }
            "libx264".to_string()
        }
    }
}

async fn test_encoder(ffmpeg: &Path, encoder: &str) -> bool {
    ffmpeg_cmd(ffmpeg)
        .args([
            "-hide_banner", "-f", "lavfi", "-i", "color=black:size=256x256:rate=30",
            "-frames:v", "3", "-c:v", encoder, "-f", "null", "-",
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .await
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Cut a clip with stream copy (fast, lossless). The cut starts on the nearest
/// keyframe before `start_sec` — recordings use a 1 s GOP so it stays close.
pub async fn trim_to_clip(ffmpeg: &Path, source: &Path, output: &Path, start_sec: f64, end_sec: f64) -> Result<(), FfmpegError> {
    let mut cmd = ffmpeg_cmd(ffmpeg);
    cmd.args([
        "-hide_banner",
        "-ss", &format!("{:.3}", start_sec),
        "-i", path_str(source),
        "-t", &format!("{:.3}", end_sec - start_sec),
        "-map", "0", "-c", "copy",
        "-avoid_negative_ts", "make_zero",
        "-movflags", "+faststart",
        "-y", path_str(output),
    ]);
    run(cmd, "clip").await
}
