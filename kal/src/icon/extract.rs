use std::path::Path;

/// Extract icons as png from paths.
pub fn extract_multiple<I, P, P2>(files: I) -> anyhow::Result<()>
where
    I: IntoIterator<Item = (P, P2)>,
    P: AsRef<Path>,
    P2: AsRef<Path>,
{
    for (src, out) in files {
        extract(src, out)?;
    }

    Ok(())
}

/// Extract icons as png from paths and cache it..
pub fn extract_multiple_cached<I, P, P2>(files: I) -> anyhow::Result<()>
where
    I: IntoIterator<Item = (P, P2)>,
    P: AsRef<Path>,
    P2: AsRef<Path>,
{
    for (src, out) in files {
        extract_cached(src, out)?;
    }

    Ok(())
}

/// Extract icon as png from path and cache it.
pub fn extract_cached<P, P2>(file: P, out: P2) -> anyhow::Result<()>
where
    P: AsRef<Path>,
    P2: AsRef<Path>,
{
    use std::time::{Duration, SystemTime};

    const DAY: Duration = Duration::from_secs(60 * 60 * 24);

    let out = out.as_ref().to_path_buf();

    if out.exists() && out.metadata()?.modified()? + DAY > SystemTime::now() {
        return Ok(());
    }

    let file = file.as_ref().to_path_buf();

    extract(file, out)
}

/// Extract icon as png from path.
#[inline]
pub fn extract<P, P2>(file: P, out: P2) -> anyhow::Result<()>
where
    P: AsRef<Path>,
    P2: AsRef<Path>,
{
    imp::extract(file, out)
}

#[cfg(windows)]
mod imp {
    use std::ffi::OsStr;
    use std::fs::File;
    use std::io::BufWriter;
    use std::ops::Deref;

    use anyhow::Context;
    use windows::core::*;
    use windows::Win32::Graphics::Gdi::*;
    use windows::Win32::Storage::FileSystem::WIN32_FIND_DATAW;
    use windows::Win32::System::Com::*;
    use windows::Win32::UI::Shell::*;
    use windows::Win32::UI::WindowsAndMessaging::*;

    use super::*;

    pub fn extract<P, P2>(file: P, out: P2) -> anyhow::Result<()>
    where
        P: AsRef<Path>,
        P2: AsRef<Path>,
    {
        let file = file.as_ref();
        let out = out.as_ref();

        let hicon = unsafe { extract_hicon(file) }?;

        let (rgba, width, height) = unsafe { hicon_to_rgba8(*hicon)? };

        save_rgba_as_png_to_disk(out, rgba, width, height)
    }

    unsafe fn extract_hicon(file: &Path) -> anyhow::Result<Owned<HICON>> {
        let file_hstr = HSTRING::from(file);
        let file_wide = file_hstr.deref();

        let len = file_wide.len().min(128);
        let mut file_wide_arr = [0_u16; 128];
        file_wide_arr[..len].copy_from_slice(&file_wide[..len]);

        let mut index = 0;

        let mut hicon = unsafe { ExtractAssociatedIconW(None, &mut file_wide_arr, &mut index) };

        // if failed and it is a shortcut, then try to resolve it
        if hicon.is_invalid() && file.extension() == Some(OsStr::new("lnk")) {
            let sl: IShellLinkW = unsafe { CoCreateInstance(&ShellLink, None, CLSCTX_ALL) }?;
            let pf = sl.cast::<IPersistFile>()?;

            unsafe { pf.Load(&file_hstr, STGM_READ) }?;

            let mut target_path = [0_u16; 128];
            let mut find_data = WIN32_FIND_DATAW::default();
            unsafe { sl.GetPath(&mut target_path, &mut find_data, 0) }?;

            let mut index = 0;
            hicon = unsafe { ExtractAssociatedIconW(None, &mut target_path, &mut index) };
        }

        if hicon.is_invalid() {
            anyhow::bail!("Failed to get HICON from {}", file.display());
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

    fn encode_png_into_writer<W: std::io::Write>(
        buf: W,
        rgba: Vec<u8>,
        width: u32,
        height: u32,
    ) -> anyhow::Result<()> {
        let file = BufWriter::new(buf);

        let mut encoder = png::Encoder::new(file, width, height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);

        let mut writer = encoder.write_header()?;
        writer.write_image_data(&rgba).map_err(Into::into)
    }

    fn save_rgba_as_png_to_disk(
        out: &Path,
        rgba: Vec<u8>,
        width: u32,
        height: u32,
    ) -> anyhow::Result<()> {
        let out = File::options()
            .create(true)
            .truncate(true)
            .write(true)
            .open(out)?;

        encode_png_into_writer(out, rgba, width, height)
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
