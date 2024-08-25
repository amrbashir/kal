use std::{
    fs::File,
    io::BufWriter,
    path::{Path, PathBuf},
};

use anyhow::Context;
use windows::{
    core::{Owned, HSTRING},
    Win32::{
        Foundation::HWND,
        Graphics::Gdi::{
            DeleteObject, GetDC, GetDIBits, GetObjectW, ReleaseDC, BITMAP, BITMAPINFOHEADER,
            BI_RGB, DIB_RGB_COLORS,
        },
        System::LibraryLoader::GetModuleHandleW,
        UI::{
            Shell::ExtractAssociatedIconW,
            WindowsAndMessaging::{GetIconInfo, HICON, ICONINFO},
        },
    },
};

/// Extract pngs from paths, using powershell
///
/// Possiple failures:
/// - When a path is a directory
pub fn extract_pngs<I>(files: I) -> anyhow::Result<()>
where
    I: IntoIterator<Item = (PathBuf, PathBuf)>,
{
    for (src, out) in files {
        extract_png(src, out)?;
    }

    Ok(())
}

pub fn extract_png<P: AsRef<Path>>(file: P, out: P) -> anyhow::Result<()> {
    let file = file.as_ref();
    let file = HSTRING::from(file);
    let file = file.as_wide();

    let mut path = [0_u16; 128];
    let len = file.len().min(128);
    for i in 0..len {
        path[i] = file[i];
    }

    let mut index = 0;

    let hicon = unsafe { ExtractAssociatedIconW(GetModuleHandleW(None)?, &mut path, &mut index) };
    let hicon = unsafe { Owned::new(hicon) };

    unsafe { save_hicon(*hicon, out)? };

    Ok(())
}

unsafe fn save_hicon<P: AsRef<Path>>(hicon: HICON, out: P) -> anyhow::Result<()> {
    let bitmap_size_i32 = i32::try_from(std::mem::size_of::<BITMAP>())?;
    let biheader_size_u32 = u32::try_from(std::mem::size_of::<BITMAPINFOHEADER>())?;

    let mut info = ICONINFO::default();
    GetIconInfo(hicon, &mut info)?;
    if !DeleteObject(info.hbmMask).as_bool() {
        return Err(windows::core::Error::from_win32().into());
    }

    let mut bitmap = BITMAP::default();
    let result = GetObjectW(
        info.hbmColor,
        bitmap_size_i32,
        Some(&mut bitmap as *mut _ as *mut _),
    );
    assert!(result == bitmap_size_i32);

    let width_u32 = u32::try_from(bitmap.bmWidth)?;
    let height_u32 = u32::try_from(bitmap.bmHeight)?;
    let width_usize = usize::try_from(bitmap.bmWidth)?;
    let height_usize = usize::try_from(bitmap.bmHeight)?;
    let buf_size = width_usize
        .checked_mul(height_usize)
        .and_then(|size| size.checked_mul(4))
        .context("failed to get buf_size")?;
    let mut buf: Vec<u8> = Vec::with_capacity(buf_size);

    let dc = GetDC(HWND::default());
    assert!(!dc.is_invalid());

    let mut bitmap_info = BITMAPINFOHEADER {
        biSize: biheader_size_u32,
        biWidth: bitmap.bmWidth,
        biHeight: -bitmap.bmHeight,
        biPlanes: 1,
        biBitCount: 32,
        biCompression: BI_RGB.0,
        biSizeImage: 0,
        biXPelsPerMeter: 0,
        biYPelsPerMeter: 0,
        biClrUsed: 0,
        biClrImportant: 0,
    };
    let result = GetDIBits(
        dc,
        info.hbmColor,
        0,
        height_u32,
        Some(buf.as_mut_ptr().cast()),
        std::ptr::addr_of_mut!(bitmap_info).cast(),
        DIB_RGB_COLORS,
    );
    assert!(result == bitmap.bmHeight);
    buf.set_len(buf.capacity());

    let result = ReleaseDC(HWND::default(), dc);
    assert!(result == 1);
    DeleteObject(info.hbmColor).unwrap();

    for chunk in buf.chunks_exact_mut(4) {
        let [b, _, r, _] = chunk else { unreachable!() };
        std::mem::swap(b, r);
    }

    let file = File::options()
        .create(true)
        .truncate(true)
        .write(true)
        .open(out)?;
    let file = BufWriter::new(file);
    let mut encoder = png::Encoder::new(file, width_u32, height_u32);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);

    let mut writer = encoder.write_header()?;
    writer.write_image_data(&buf).map_err(Into::into)
}
