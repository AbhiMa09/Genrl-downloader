use std::fs;
use std::path::Path;
use std::process::Command;

use crate::error::{Error, Result};
use crate::settings::{IconMethod, IconStyle};

use super::types::DIRECTORY_FILE;

/// Standard IEEE 802.3 CRC32 calculation.
fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for &b in data {
        crc ^= b as u32;
        for _ in 0..8 {
            if (crc & 1) != 0 {
                crc = (crc >> 1) ^ 0xEDB8_8320;
            } else {
                crc >>= 1;
            }
        }
    }
    !crc
}

/// Generate a valid 512x512 PNG with a solid background color (pure Rust fallback).
pub fn write_solid_png(
    path: &Path,
    width: u32,
    height: u32,
    r: u8,
    g: u8,
    b: u8,
) -> std::io::Result<()> {
    use std::io::Write;

    let mut file = fs::File::create(path)?;
    // 8-byte PNG signature
    file.write_all(b"\x89PNG\r\n\x1a\n")?;

    // Helper to write a PNG chunk with CRC
    let write_chunk = |file: &mut fs::File, tag: &[u8; 4], data: &[u8]| -> std::io::Result<()> {
        let len = data.len() as u32;
        file.write_all(&len.to_be_bytes())?;
        file.write_all(tag)?;
        file.write_all(data)?;
        let mut crc_buf = Vec::with_capacity(4 + data.len());
        crc_buf.extend_from_slice(tag);
        crc_buf.extend_from_slice(data);
        let crc_val = crc32(&crc_buf);
        file.write_all(&crc_val.to_be_bytes())?;
        Ok(())
    };

    // IHDR chunk
    let mut ihdr = Vec::with_capacity(13);
    ihdr.extend_from_slice(&width.to_be_bytes());
    ihdr.extend_from_slice(&height.to_be_bytes());
    ihdr.push(8); // bit depth
    ihdr.push(2); // color type: RGB
    ihdr.push(0); // compression
    ihdr.push(0); // filter
    ihdr.push(0); // interlace
    write_chunk(&mut file, b"IHDR", &ihdr)?;

    // Construct raw scanlines: filter byte 0 followed by width * 3 bytes
    let line_len = 1 + (width as usize) * 3;
    let mut row = Vec::with_capacity(line_len);
    row.push(0); // filter: None
    for _ in 0..width {
        row.push(r);
        row.push(g);
        row.push(b);
    }
    let mut raw = Vec::with_capacity(line_len * (height as usize));
    for _ in 0..height {
        raw.extend_from_slice(&row);
    }

    // zlib wrapping for IDAT
    let mut zlib_data = Vec::new();
    zlib_data.push(0x78); // zlib header
    zlib_data.push(0x01); // compression level

    // Split raw into uncompressed DEFLATE blocks (max 65535 bytes per block)
    let chunks = raw.chunks(65535);
    let chunk_count = chunks.len();
    for (i, chunk) in chunks.enumerate() {
        let is_last = i == chunk_count - 1;
        zlib_data.push(if is_last { 0x01 } else { 0x00 });
        let len = chunk.len() as u16;
        let nlen = !len;
        zlib_data.extend_from_slice(&len.to_le_bytes());
        zlib_data.extend_from_slice(&nlen.to_le_bytes());
        zlib_data.extend_from_slice(chunk);
    }

    // Adler-32 checksum
    let mut s1 = 1u32;
    let mut s2 = 0u32;
    for &byte in &raw {
        s1 = (s1 + byte as u32) % 65521;
        s2 = (s2 + s1) % 65521;
    }
    let adler = (s2 << 16) | s1;
    zlib_data.extend_from_slice(&adler.to_be_bytes());

    write_chunk(&mut file, b"IDAT", &zlib_data)?;
    write_chunk(&mut file, b"IEND", &[])?;

    Ok(())
}

// ---------------------------------------------------------------------------
// Desktop Folder Icons
// ---------------------------------------------------------------------------

/// Create the folder.png icon file (square 512x512).
pub fn create_folder_png(
    poster_src: &Path,
    folder_png_dst: &Path,
    _style: IconStyle,
) -> Result<()> {
    // Try ImageMagick (magick or convert) first if available
    let binaries = ["magick", "convert"];
    for bin in binaries {
        let status = Command::new(bin)
            .arg(poster_src)
            .args([
                "-resize", "512x512^", "-gravity", "center", "-extent", "512x512",
            ])
            .arg(folder_png_dst)
            .status();
        if let Ok(st) = status {
            if st.success() && folder_png_dst.exists() {
                return Ok(());
            }
        }
    }

    // Fallback: If poster_src is already a valid file, try copying it directly
    if poster_src.exists() && fs::copy(poster_src, folder_png_dst).is_ok() {
        return Ok(());
    }

    // Fallback: Generate a dark violet square PNG (64x64)
    write_solid_png(folder_png_dst, 64, 64, 30, 20, 50).map_err(Error::Io)?;
    Ok(())
}

/// Apply folder icon based on the desktop environment setting.
pub fn apply_folder_icon(
    method: IconMethod,
    show_dir: &Path,
    folder_png_path: &Path,
) -> Result<()> {
    match method {
        IconMethod::Off => Ok(()),
        IconMethod::Directory => write_directory_file(show_dir),
        IconMethod::Gio => set_gio_custom_icon(show_dir, folder_png_path),
        IconMethod::Auto => {
            let desktop = std::env::var("XDG_CURRENT_DESKTOP")
                .unwrap_or_default()
                .to_ascii_lowercase();
            if desktop.contains("gnome") {
                if set_gio_custom_icon(show_dir, folder_png_path).is_err() {
                    let _ = write_directory_file(show_dir);
                }
            } else {
                let _ = write_directory_file(show_dir);
            }
            Ok(())
        }
    }
}

fn write_directory_file(show_dir: &Path) -> Result<()> {
    let directory_file = show_dir.join(DIRECTORY_FILE);
    let contents = "[Desktop Entry]\nIcon=folder.png\n";
    fs::write(&directory_file, contents)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(&directory_file, fs::Permissions::from_mode(0o644));
    }
    Ok(())
}

fn set_gio_custom_icon(show_dir: &Path, folder_png_path: &Path) -> Result<()> {
    let uri = format!("file://{}", folder_png_path.display());
    let status = Command::new("gio")
        .args(["set", "-t", "string"])
        .arg(show_dir)
        .arg("metadata::custom-icon")
        .arg(&uri)
        .status();
    match status {
        Ok(st) if st.success() => Ok(()),
        Ok(st) => Err(Error::Other(format!("gio set failed with status {st}"))),
        Err(e) => Err(Error::Io(e)),
    }
}
