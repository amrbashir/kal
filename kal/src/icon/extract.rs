use std::path::Path;
use std::time::{Duration, SystemTime};

use image::RgbaImage;

/// Extract icon as png from `path` and saves it into `out`.
#[inline]
pub fn extract<P, P2>(file: P, out: P2) -> anyhow::Result<()>
where
    P: AsRef<Path>,
    P2: AsRef<Path>,
{
    imp::extract(file, out)
}

/// Same as [`extract`] but avoids extracting if out has not been modified in the past 24 hours.
pub fn extract_cached<P, P2>(path: P, out: P2) -> anyhow::Result<()>
where
    P: AsRef<Path>,
    P2: AsRef<Path>,
{
    const DAY: Duration = Duration::from_secs(60 * 60 * 24);

    let out = out.as_ref().to_path_buf();

    if out.exists() && out.metadata()?.modified()? + DAY > SystemTime::now() {
        return Ok(());
    }

    extract(path, out)
}

/// Extract two icons from `bottom` and `top` then overlays `top` on `bottom`
/// with half the size then saves it into `out`.
pub fn extract_overlayed<P, P2, P3>(bottom: P, top: P2, out: P3) -> anyhow::Result<()>
where
    P: AsRef<Path>,
    P2: AsRef<Path>,
    P3: AsRef<Path>,
{
    let mut bottom = imp::extract_image(bottom)?;

    let top = imp::extract_image(top)?;
    let second = image::DynamicImage::ImageRgba8(top);

    let top = second.thumbnail(bottom.width() / 2, bottom.height() / 2);

    let x = bottom.width() - bottom.width() / 2;
    let y = bottom.height() - bottom.height() / 2;
    image::imageops::overlay(&mut bottom, &top, x.into(), y.into());

    bottom
        .save_with_format(out, image::ImageFormat::Png)
        .map_err(Into::into)
}

/// Same as [`extract_overlayed`] but avoids extracting if out has not been modified in the past 24 hours.
pub fn extract_overlayed_cached<P, P2, P3>(bottom: P, top: P2, out: P3) -> anyhow::Result<()>
where
    P: AsRef<Path>,
    P2: AsRef<Path>,
    P3: AsRef<Path>,
{
    const DAY: Duration = Duration::from_secs(60 * 60 * 24);

    let out = out.as_ref().to_path_buf();

    if out.exists() && out.metadata()?.modified()? + DAY > SystemTime::now() {
        return Ok(());
    }

    extract_overlayed(bottom, top, out)
}

#[cfg(windows)]
mod imp {
    use std::ffi::OsStr;
    use std::ops::Deref;

    use anyhow::Context;
    use windows::core::*;
    use windows::Win32::Graphics::Gdi::*;
    use windows::Win32::Storage::FileSystem::WIN32_FIND_DATAW;
    use windows::Win32::System::Com::*;
    use windows::Win32::UI::Shell::*;
    use windows::Win32::UI::WindowsAndMessaging::*;

    use super::*;

    pub fn extract<P, P2>(path: P, out: P2) -> anyhow::Result<()>
    where
        P: AsRef<Path>,
        P2: AsRef<Path>,
    {
        let path = path.as_ref();
        let out = out.as_ref();

        let hicon = unsafe { extract_hicon(path) }?;

        let (rgba, width, height) = unsafe { hicon_to_rgba8(*hicon)? };

        save_rgba_as_png_to_disk(out, rgba, width, height)
    }

    pub fn extract_image<P>(path: P) -> anyhow::Result<RgbaImage>
    where
        P: AsRef<Path>,
    {
        let path = path.as_ref();

        let hicon = unsafe { extract_hicon(path) }?;

        let (rgba, width, height) = unsafe { hicon_to_rgba8(*hicon)? };

        RgbaImage::from_vec(width, height, rgba).context("Failed to construct RgbaImage")
    }

    unsafe fn extract_hicon(path: &Path) -> anyhow::Result<Owned<HICON>> {
        let path_hstr = HSTRING::from(path);
        let path_wide = path_hstr.deref();

        let len = path_wide.len().min(128);
        let mut path_wide_arr = [0_u16; 128];
        path_wide_arr[..len].copy_from_slice(&path_wide[..len]);

        let mut index = 0;

        let mut hicon = unsafe { ExtractAssociatedIconW(None, &mut path_wide_arr, &mut index) };

        // if failed and it is a shortcut, then try to resolve it
        if hicon.is_invalid() && path.extension() == Some(OsStr::new("lnk")) {
            let sl: IShellLinkW = unsafe { CoCreateInstance(&ShellLink, None, CLSCTX_ALL) }?;
            let pf = sl.cast::<IPersistFile>()?;

            unsafe { pf.Load(&path_hstr, STGM_READ) }?;

            let mut target_path = [0_u16; 128];
            let mut find_data = WIN32_FIND_DATAW::default();
            unsafe { sl.GetPath(&mut target_path, &mut find_data, 0) }?;

            let mut index = 0;
            hicon = unsafe { ExtractAssociatedIconW(None, &mut target_path, &mut index) };
        }

        if hicon.is_invalid() {
            anyhow::bail!("Failed to get HICON from {}", path.display());
        }

        Ok(Owned::new(hicon))
    }

    unsafe fn hicon_to_rgba8(hicon: HICON) -> anyhow::Result<(Vec<u8>, u32, u32)> {
        let bitmap_size_i32 = i32::try_from(std::mem::size_of::<BITMAP>())?;
        let biheader_size_u32 = u32::try_from(std::mem::size_of::<BITMAPINFOHEADER>())?;

        let mut info = ICONINFO::default();
        GetIconInfo(hicon, &mut info)?;
        if !DeleteObject(info.hbmMask.into()).as_bool() {
            return Err(windows::core::Error::from_win32().into());
        }

        let mut bitmap = BITMAP::default();
        let result = GetObjectW(
            info.hbmColor.into(),
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

        let dc = GetDC(None);
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

        let result = ReleaseDC(None, dc);
        assert!(result == 1);
        DeleteObject(info.hbmColor.into()).ok()?;

        for chunk in buf.chunks_exact_mut(4) {
            let [b, _, r, _] = chunk else { unreachable!() };
            std::mem::swap(b, r);
        }

        Ok((buf, width_u32, height_u32))
    }

    fn save_rgba_as_png_to_disk(
        out: &Path,
        rgba: Vec<u8>,
        width: u32,
        height: u32,
    ) -> anyhow::Result<()> {
        RgbaImage::from_vec(width, height, rgba)
            .context("Failed to construct RgbaImage")?
            .save_with_format(out, image::ImageFormat::Png)
            .map_err(Into::into)
    }
}

#[cfg(not(windows))]
mod imp {
    use super::*;

    pub fn extract<P, P2>(file: P, out: P2) -> anyhow::Result<()>
    where
        P: AsRef<Path>,
        P2: AsRef<Path>,
    {
        unimplemented!()
    }
}
