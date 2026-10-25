fn main() {
    tauri_build::build();

    // Patch the bundled FFmpeg binary: change PE subsystem from CONSOLE (3) to
    // WINDOWS_GUI (2) so Windows never allocates a console window for it.
    // FFmpeg works fine as a GUI subsystem app because we pipe all its stdio.
    #[cfg(target_os = "windows")]
    patch_ffmpeg_subsystem();
}

#[cfg(target_os = "windows")]
fn patch_ffmpeg_subsystem() {
    let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let ffmpeg = std::path::Path::new(&manifest)
        .join("binaries")
        .join("ffmpeg-x86_64-pc-windows-msvc.exe");

    if !ffmpeg.exists() {
        return;
    }

    let mut bytes = match std::fs::read(&ffmpeg) {
        Ok(b) => b,
        Err(_) => return,
    };

    if bytes.len() < 0x42 || bytes[0] != b'M' || bytes[1] != b'Z' {
        return; // not a PE file
    }

    let e_lfanew = u32::from_le_bytes([bytes[0x3C], bytes[0x3D], bytes[0x3E], bytes[0x3F]]) as usize;
    let opt_hdr = e_lfanew.saturating_add(24); // PE sig (4) + COFF header (20)

    if opt_hdr + 94 > bytes.len() {
        return;
    }

    let magic = u16::from_le_bytes([bytes[opt_hdr], bytes[opt_hdr + 1]]);
    // Subsystem is at +68 from Optional Header start in BOTH PE32 and PE32+
    let sub_off = match magic {
        0x010b | 0x020b => opt_hdr + 68,
        _ => return,
    };

    if sub_off + 1 >= bytes.len() {
        return;
    }

    let subsystem = u16::from_le_bytes([bytes[sub_off], bytes[sub_off + 1]]);
    if subsystem == 3 {
        // Change CONSOLE → WINDOWS_GUI
        bytes[sub_off] = 2;
        bytes[sub_off + 1] = 0;
        if std::fs::write(&ffmpeg, &bytes).is_ok() {
            println!("cargo:warning=ffmpeg.exe PE subsystem patched CONSOLE→WINDOWS_GUI (no more console window)");
        }
    }
}
