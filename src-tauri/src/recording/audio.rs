// WASAPI audio capture — one thread per source, writes a 16-bit PCM WAV.
//
// Sources:
//   - Loopback: the default OUTPUT device (everything you hear, incl. the game)
//   - Mic: an INPUT device (default one, or matched by friendly name)
//
// Sync model: the WAV timeline starts exactly at `AudioTrack::start` (an
// `Instant` taken right after `IAudioClient::Start`). WASAPI loopback delivers
// NO packets while nothing is playing, so we use each packet's QPC timestamp
// to detect holes and fill them with silence. Without this, every silent
// moment shortened the WAV and the sound drifted ahead of the video.

use std::io::{BufWriter, Seek, SeekFrom, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::Arc;
use std::time::Instant;

use windows::core::GUID;
use windows::Win32::Media::Audio::{
    eCapture, eConsole, eRender, IAudioCaptureClient, IAudioClient, IMMDevice,
    IMMDeviceEnumerator, MMDeviceEnumerator, AUDCLNT_SHAREMODE_SHARED,
    AUDCLNT_STREAMFLAGS_LOOPBACK, DEVICE_STATE_ACTIVE, WAVEFORMATEX,
};
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoTaskMemFree, CoUninitialize, CLSCTX_ALL,
    COINIT_MULTITHREADED, STGM_READ,
};
use windows::Win32::System::Performance::{QueryPerformanceCounter, QueryPerformanceFrequency};
use windows::Win32::System::Variant::VT_LPWSTR;
use windows::Win32::UI::Shell::PropertiesSystem::PROPERTYKEY;

/// AUDCLNT_BUFFERFLAGS_*
const FLAG_SILENT: u32 = 0x2;
const FLAG_TIMESTAMP_ERROR: u32 = 0x4;

/// KSDATAFORMAT_SUBTYPE_IEEE_FLOAT
const SUBTYPE_FLOAT: GUID = GUID::from_u128(0x00000003_0000_0010_8000_00aa00389b71);

/// Holes shorter than this are ignored (normal packet jitter).
const GAP_TOLERANCE_MS: u64 = 40;

#[derive(Debug, Clone)]
pub enum AudioSource {
    Loopback,
    /// `None` = default microphone.
    Mic(Option<String>),
}

/// A finished recording: the WAV file and the instant its first sample maps to.
pub struct AudioTrack {
    pub path: PathBuf,
    pub start: Instant,
}

pub struct AudioCapture {
    stop: Arc<AtomicBool>,
    thread: Option<std::thread::JoinHandle<Result<(), String>>>,
    wav_path: PathBuf,
    start: Instant,
}

impl AudioCapture {
    /// Starts capturing. Blocks until the device is actually running so the
    /// returned start instant is accurate.
    pub fn start(source: AudioSource, wav_path: PathBuf) -> Result<Self, String> {
        let stop = Arc::new(AtomicBool::new(false));
        let stop2 = Arc::clone(&stop);
        let path = wav_path.clone();
        let (ready_tx, ready_rx) = mpsc::channel::<Result<Instant, String>>();

        let thread = std::thread::spawn(move || {
            capture_thread(source, path, stop2, ready_tx).map_err(|e| e.to_string())
        });

        match ready_rx.recv() {
            Ok(Ok(start)) => Ok(Self { stop, thread: Some(thread), wav_path, start }),
            Ok(Err(e)) => {
                let _ = thread.join();
                Err(e)
            }
            Err(_) => {
                let _ = thread.join();
                Err("audio thread exited before starting".into())
            }
        }
    }

    pub fn stop(mut self) -> Option<AudioTrack> {
        self.stop.store(true, Ordering::Release);
        let t = self.thread.take()?;
        match t.join() {
            Ok(Ok(())) => Some(AudioTrack { path: self.wav_path.clone(), start: self.start }),
            Ok(Err(e)) => {
                log::error!("Audio capture failed: {}", e);
                None
            }
            Err(_) => {
                log::error!("Audio capture thread panicked");
                None
            }
        }
    }
}

impl Drop for AudioCapture {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
    }
}

/// Enumerate active microphone (capture) devices by friendly name.
pub fn list_mic_devices() -> Vec<String> {
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        let Ok(enumerator) = CoCreateInstance::<_, IMMDeviceEnumerator>(&MMDeviceEnumerator, None, CLSCTX_ALL) else {
            return Vec::new();
        };
        let Ok(collection) = enumerator.EnumAudioEndpoints(eCapture, DEVICE_STATE_ACTIVE) else {
            return Vec::new();
        };
        let count = collection.GetCount().unwrap_or(0);
        (0..count)
            .filter_map(|i| collection.Item(i).ok())
            .filter_map(|d| device_friendly_name(&d))
            .collect()
    }
}

unsafe fn device_friendly_name(device: &IMMDevice) -> Option<String> {
    // PKEY_Device_FriendlyName = {a45c254e-df1c-4efd-8020-67d146a850e0}, pid 14
    let pkey = PROPERTYKEY {
        fmtid: GUID::from_u128(0xa45c254e_df1c_4efd_8020_67d146a850e0),
        pid: 14,
    };
    let store = device.OpenPropertyStore(STGM_READ).ok()?;
    let prop = store.GetValue(&pkey).ok()?;
    let raw = prop.as_raw();
    if raw.Anonymous.Anonymous.vt != VT_LPWSTR.0 {
        return None;
    }
    let pwsz = raw.Anonymous.Anonymous.Anonymous.pwszVal;
    if pwsz.is_null() {
        return None;
    }
    let len = (0..).take_while(|&i| *pwsz.add(i) != 0).count();
    let name = String::from_utf16_lossy(std::slice::from_raw_parts(pwsz, len));
    (!name.is_empty()).then_some(name)
}

unsafe fn find_mic(enumerator: &IMMDeviceEnumerator, name: &str) -> Option<IMMDevice> {
    let collection = enumerator.EnumAudioEndpoints(eCapture, DEVICE_STATE_ACTIVE).ok()?;
    let count = collection.GetCount().ok()?;
    (0..count)
        .filter_map(|i| collection.Item(i).ok())
        .find(|d| device_friendly_name(d).as_deref() == Some(name))
}

/// Current QPC value converted to 100-ns units (same unit WASAPI uses).
fn qpc_100ns(freq: i64) -> u64 {
    let mut now = 0i64;
    unsafe {
        let _ = QueryPerformanceCounter(&mut now);
    }
    (now as i128 * 10_000_000 / freq as i128) as u64
}

#[derive(Clone, Copy)]
enum SampleFmt {
    F32,
    I16,
    I24,
    I32,
}

fn capture_thread(
    source: AudioSource,
    path: PathBuf,
    stop: Arc<AtomicBool>,
    ready: mpsc::Sender<Result<Instant, String>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    unsafe {
        if let Err(e) = CoInitializeEx(None, COINIT_MULTITHREADED).ok() {
            let _ = ready.send(Err(e.to_string()));
            return Err(e.into());
        }
        let res = capture_loop(source, path, stop, &ready);
        if let Err(e) = &res {
            // No-op if we already reported success.
            let _ = ready.send(Err(e.to_string()));
        }
        CoUninitialize();
        res
    }
}

unsafe fn capture_loop(
    source: AudioSource,
    path: PathBuf,
    stop: Arc<AtomicBool>,
    ready: &mpsc::Sender<Result<Instant, String>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let enumerator: IMMDeviceEnumerator = CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;

    let (device, stream_flags) = match &source {
        AudioSource::Loopback => (enumerator.GetDefaultAudioEndpoint(eRender, eConsole)?, AUDCLNT_STREAMFLAGS_LOOPBACK),
        AudioSource::Mic(name) => {
            let dev = match name {
                Some(n) => match find_mic(&enumerator, n) {
                    Some(d) => d,
                    None => {
                        log::warn!("Mic '{}' not found — using default microphone", n);
                        enumerator.GetDefaultAudioEndpoint(eCapture, eConsole)?
                    }
                },
                None => enumerator.GetDefaultAudioEndpoint(eCapture, eConsole)?,
            };
            (dev, 0)
        }
    };

    let audio_client: IAudioClient = device.Activate(CLSCTX_ALL, None)?;

    let mix_ptr = audio_client.GetMixFormat()?;
    let mix: WAVEFORMATEX = *mix_ptr;
    let channels = mix.nChannels as u32;
    let sample_rate = mix.nSamplesPerSec;
    let bits = mix.wBitsPerSample;
    let tag = mix.wFormatTag;

    let is_float = match tag {
        3 => true,
        0xFFFE => {
            // WAVEFORMATEXTENSIBLE.SubFormat lives at byte offset 24.
            let sub = std::ptr::read_unaligned((mix_ptr as *const u8).add(24) as *const GUID);
            sub == SUBTYPE_FLOAT
        }
        _ => false,
    };
    let fmt = match (is_float, bits) {
        (true, 32) => SampleFmt::F32,
        (false, 16) => SampleFmt::I16,
        (false, 24) => SampleFmt::I24,
        (false, 32) => SampleFmt::I32,
        _ => {
            CoTaskMemFree(Some(mix_ptr as *const _));
            return Err(format!("unsupported mix format: tag={:#x} bits={}", tag, bits).into());
        }
    };
    let in_bytes_per_sample = (bits / 8) as usize;

    // 100 ms shared-mode buffer (100-ns units)
    let init = audio_client.Initialize(AUDCLNT_SHAREMODE_SHARED, stream_flags, 1_000_000, 0, mix_ptr, None);
    CoTaskMemFree(Some(mix_ptr as *const _));
    init?;

    let capture_client: IAudioCaptureClient = audio_client.GetService()?;

    const OUT_BITS: u16 = 16;
    let out_frame_bytes = channels as usize * 2;
    let file = std::fs::File::create(&path)?;
    let mut out = BufWriter::with_capacity(256 * 1024, file);
    write_wav_header(&mut out, sample_rate, channels as u16, OUT_BITS, 0)?;

    let mut freq = 0i64;
    QueryPerformanceFrequency(&mut freq)?;

    audio_client.Start()?;
    let start_instant = Instant::now();
    let start_qpc = qpc_100ns(freq);
    let _ = ready.send(Ok(start_instant));

    log::info!(
        "Audio {:?} started: {} Hz, {} ch, {} bits{}",
        source, sample_rate, channels, bits, if is_float { " float" } else { "" }
    );

    let frames_for = |hns: u64| -> u64 { hns * sample_rate as u64 / 10_000_000 };
    let tolerance = frames_for(GAP_TOLERANCE_MS * 10_000);
    let mut written: u64 = 0;
    let mut silence_buf: Vec<u8> = Vec::new();
    let mut pcm: Vec<u8> = Vec::new();

    let mut write_silence = |out: &mut BufWriter<std::fs::File>, frames: u64, written: &mut u64| -> std::io::Result<()> {
        let mut left = frames as usize * out_frame_bytes;
        if silence_buf.is_empty() {
            silence_buf = vec![0u8; 64 * 1024];
        }
        while left > 0 {
            let n = left.min(silence_buf.len());
            out.write_all(&silence_buf[..n])?;
            left -= n;
        }
        *written += frames;
        Ok(())
    };

    while !stop.load(Ordering::Acquire) {
        // Device errors (headset unplugged, default device changed…) end the
        // capture but we still finalize the WAV so the audio isn't lost.
        let next = match capture_client.GetNextPacketSize() {
            Ok(n) => n,
            Err(e) => {
                log::error!("Audio {:?} device error, stopping capture: {}", source, e);
                break;
            }
        };
        if next == 0 {
            std::thread::sleep(std::time::Duration::from_millis(5));
            continue;
        }

        let mut data: *mut u8 = std::ptr::null_mut();
        let mut frames: u32 = 0;
        let mut flags: u32 = 0;
        let mut qpc_pos: u64 = 0;
        if let Err(e) = capture_client.GetBuffer(&mut data, &mut frames, &mut flags, None, Some(&mut qpc_pos)) {
            log::error!("Audio {:?} GetBuffer failed, stopping capture: {}", source, e);
            break;
        }

        // Fill the hole between what we wrote and where this packet belongs.
        if flags & FLAG_TIMESTAMP_ERROR == 0 && qpc_pos > start_qpc {
            let expected = frames_for(qpc_pos - start_qpc);
            if expected > written + tolerance {
                write_silence(&mut out, expected - written, &mut written)?;
            }
        }

        if frames > 0 {
            if flags & FLAG_SILENT != 0 || data.is_null() {
                write_silence(&mut out, frames as u64, &mut written)?;
            } else {
                let samples = frames as usize * channels as usize;
                let raw = std::slice::from_raw_parts(data, samples * in_bytes_per_sample);
                pcm.clear();
                pcm.reserve(samples * 2);
                convert_to_i16(raw, fmt, &mut pcm);
                out.write_all(&pcm)?;
                written += frames as u64;
            }
        }

        let _ = capture_client.ReleaseBuffer(frames);
    }

    // Pad the tail up to "now" so the WAV covers the whole wall-clock span.
    let expected = frames_for(qpc_100ns(freq).saturating_sub(start_qpc));
    if expected > written {
        write_silence(&mut out, expected - written, &mut written)?;
    }

    let _ = audio_client.Stop();

    let data_bytes = (written * out_frame_bytes as u64).min(u32::MAX as u64 - 64) as u32;
    let mut file = out.into_inner().map_err(|e| e.into_error())?;
    file.seek(SeekFrom::Start(0))?;
    write_wav_header(&mut file, sample_rate, channels as u16, OUT_BITS, data_bytes)?;

    log::info!(
        "Audio {:?}: {:.1}s written ({} frames @ {} Hz)",
        source,
        written as f64 / sample_rate as f64,
        written,
        sample_rate
    );
    Ok(())
}

fn convert_to_i16(raw: &[u8], fmt: SampleFmt, out: &mut Vec<u8>) {
    match fmt {
        SampleFmt::F32 => {
            for c in raw.chunks_exact(4) {
                let s = f32::from_le_bytes([c[0], c[1], c[2], c[3]]);
                let v = (s.clamp(-1.0, 1.0) * 32767.0) as i16;
                out.extend_from_slice(&v.to_le_bytes());
            }
        }
        SampleFmt::I16 => out.extend_from_slice(raw),
        SampleFmt::I24 => {
            for c in raw.chunks_exact(3) {
                out.extend_from_slice(&[c[1], c[2]]);
            }
        }
        SampleFmt::I32 => {
            for c in raw.chunks_exact(4) {
                out.extend_from_slice(&[c[2], c[3]]);
            }
        }
    }
}

fn write_wav_header<W: Write>(f: &mut W, sample_rate: u32, channels: u16, bits: u16, data_bytes: u32) -> std::io::Result<()> {
    let block_align = channels * bits / 8;
    let byte_rate = sample_rate * block_align as u32;
    f.write_all(b"RIFF")?;
    f.write_all(&(36 + data_bytes).to_le_bytes())?;
    f.write_all(b"WAVE")?;
    f.write_all(b"fmt ")?;
    f.write_all(&16u32.to_le_bytes())?;
    f.write_all(&1u16.to_le_bytes())?; // PCM
    f.write_all(&channels.to_le_bytes())?;
    f.write_all(&sample_rate.to_le_bytes())?;
    f.write_all(&byte_rate.to_le_bytes())?;
    f.write_all(&block_align.to_le_bytes())?;
    f.write_all(&bits.to_le_bytes())?;
    f.write_all(b"data")?;
    f.write_all(&data_bytes.to_le_bytes())?;
    Ok(())
}
