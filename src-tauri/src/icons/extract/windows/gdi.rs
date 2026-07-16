use super::super::ExtractError;
use super::handles::{BitmapSelection, MemoryDc, ScreenDc};
use image::RgbaImage;
use windows::core::Owned;
use windows::Win32::Graphics::Gdi::{
    CreateCompatibleBitmap, GetDIBits, GetObjectW, PatBlt, BITMAP, BITMAPINFO, BITMAPINFOHEADER,
    BI_RGB, BLACKNESS, DIB_RGB_COLORS, HBITMAP, HGDIOBJ,
};
use windows::Win32::UI::WindowsAndMessaging::{DrawIconEx, DI_NORMAL, HICON};

pub(super) fn hicon_to_rgba(icon: HICON, size: u32) -> Result<RgbaImage, ExtractError> {
    if icon.is_invalid() {
        return Err(ExtractError::Api("invalid icon handle".into()));
    }
    let extent =
        i32::try_from(size).map_err(|_| ExtractError::Api("icon size exceeds i32".into()))?;
    let screen = ScreenDc::acquire().map_err(ExtractError::Api)?;
    // SAFETY: [Category 8 - FFI boundary] the screen DC is checked and both
    // dimensions are positive `i32`; the returned bitmap is checked below.
    let raw_bitmap = unsafe { CreateCompatibleBitmap(screen.handle(), extent, extent) };
    if raw_bitmap.is_invalid() {
        return Err(ExtractError::Api("CreateCompatibleBitmap failed".into()));
    }
    // SAFETY: [Category 12 - invalid/double free] CreateCompatibleBitmap
    // returned one caller-owned valid HBITMAP and this is its sole owner.
    let bitmap: Owned<HBITMAP> = unsafe { Owned::new(raw_bitmap) };
    let memory = MemoryDc::compatible_with(screen.handle()).map_err(ExtractError::Api)?;
    let selection = BitmapSelection::select(&memory, &bitmap).map_err(ExtractError::Api)?;
    // SAFETY: [Category 8 - FFI boundary] the selected bitmap and memory DC are
    // live and the rectangle lies within the requested bitmap dimensions.
    let _ = unsafe { PatBlt(memory.handle(), 0, 0, extent, extent, BLACKNESS) };
    // SAFETY: [Category 8 - FFI boundary] the icon, selected destination DC,
    // and dimensions are checked and stay live for this synchronous draw.
    let draw_result = unsafe {
        DrawIconEx(
            memory.handle(),
            0,
            0,
            icon,
            extent,
            extent,
            0,
            None,
            DI_NORMAL,
        )
    };
    selection.restore().map_err(ExtractError::Api)?;
    if draw_result.is_err() {
        return Err(ExtractError::Api("DrawIconEx failed".into()));
    }
    hbitmap_to_rgba(*bitmap, size).map_err(ExtractError::Api)
}

pub(super) fn hbitmap_to_rgba(bitmap: HBITMAP, size: u32) -> Result<RgbaImage, String> {
    if bitmap.is_invalid() {
        return Err("invalid bitmap handle".into());
    }
    let mut object = BITMAP::default();
    let object_size =
        i32::try_from(std::mem::size_of::<BITMAP>()).map_err(|_| "BITMAP size exceeds i32")?;
    // SAFETY: [Category 8 - FFI boundary] `object` is aligned, initialized, and
    // writable for exactly object_size bytes; `bitmap` is checked and live.
    let object_result = unsafe {
        GetObjectW(
            HGDIOBJ(bitmap.0),
            object_size,
            Some(std::ptr::from_mut(&mut object).cast()),
        )
    };
    if object_result == 0 {
        return Err("GetObjectW failed".into());
    }
    let width = positive_dimension_or(object.bmWidth, size);
    let height = positive_dimension_or(object.bmHeight, size);
    let width_i32 = i32::try_from(width).map_err(|_| "bitmap width exceeds i32")?;
    let height_i32 = i32::try_from(height).map_err(|_| "bitmap height exceeds i32")?;
    let top_down_height = height_i32
        .checked_neg()
        .ok_or("bitmap height cannot be negated")?;
    let mut info = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: u32::try_from(std::mem::size_of::<BITMAPINFOHEADER>())
                .map_err(|_| "BITMAPINFOHEADER size exceeds u32")?,
            biWidth: width_i32,
            biHeight: top_down_height,
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0,
            ..Default::default()
        },
        ..Default::default()
    };
    let byte_len = checked_bitmap_byte_len(width, height)?;
    let mut pixels = Vec::new();
    pixels
        .try_reserve_exact(byte_len)
        .map_err(|_| "bitmap buffer allocation failed")?;
    pixels.resize(byte_len, 0_u8);
    let screen = ScreenDc::acquire()?;
    let memory = MemoryDc::compatible_with(screen.handle())?;
    // SAFETY: [Categories 8 and 10 - FFI boundary/out of bounds] `pixels` is a
    // uniquely borrowed initialized buffer of checked width*height*4 bytes;
    // bitmap/info/DC handles remain live and the bitmap is not selected.
    let lines = unsafe {
        GetDIBits(
            memory.handle(),
            bitmap,
            0,
            height,
            Some(pixels.as_mut_ptr().cast()),
            &mut info,
            DIB_RGB_COLORS,
        )
    };
    if lines == 0 {
        return Err("GetDIBits failed".into());
    }
    let rgba = bgra_to_rgba(&pixels);
    RgbaImage::from_raw(width, height, rgba).ok_or_else(|| "invalid rgba buffer".into())
}

fn positive_dimension_or(value: i32, fallback: u32) -> u32 {
    match u32::try_from(value) {
        Ok(dimension) if dimension > 0 => dimension,
        Ok(_) | Err(_) => fallback,
    }
}

fn checked_bitmap_byte_len(width: u32, height: u32) -> Result<usize, String> {
    let width = usize::try_from(width).map_err(|_| "bitmap width exceeds usize")?;
    let height = usize::try_from(height).map_err(|_| "bitmap height exceeds usize")?;
    width
        .checked_mul(height)
        .and_then(|pixels| pixels.checked_mul(4))
        .ok_or_else(|| "bitmap byte length overflow".into())
}

fn bgra_to_rgba(pixels: &[u8]) -> Vec<u8> {
    let has_alpha = pixels.chunks_exact(4).any(|pixel| pixel[3] != 0);
    let mut rgba = Vec::with_capacity(pixels.len());
    for pixel in pixels.chunks_exact(4) {
        let [blue, green, red, source_alpha] = [pixel[0], pixel[1], pixel[2], pixel[3]];
        let alpha = if has_alpha {
            source_alpha
        } else if red == 0 && green == 0 && blue == 0 {
            0
        } else {
            255
        };
        rgba.extend_from_slice(&[red, green, blue, alpha]);
    }
    rgba
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_bitmap_length_when_dimensions_overflow() {
        // Given
        let width = u32::MAX;
        let height = u32::MAX;

        // When
        let result = checked_bitmap_byte_len(width, height);

        // Then
        assert_eq!(result, Err("bitmap byte length overflow".into()));
    }

    #[test]
    fn converts_bgra_and_synthesizes_alpha_when_source_has_none() {
        // Given
        let pixels = [0_u8, 0, 0, 0, 30, 20, 10, 0];

        // When
        let rgba = bgra_to_rgba(&pixels);

        // Then
        assert_eq!(rgba, [0, 0, 0, 0, 10, 20, 30, 255]);
    }
}
