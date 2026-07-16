use super::super::ExtractError;
use super::gdi;
use image::RgbaImage;
use std::ffi::OsStr;
use std::marker::PhantomData;
use std::os::windows::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use windows::core::{Interface, Owned, PCWSTR, PWSTR};
use windows::Win32::Foundation::SIZE;
use windows::Win32::Graphics::Gdi::HBITMAP;
use windows::Win32::System::Com::{CoInitializeEx, CoUninitialize, COINIT_APARTMENTTHREADED};
use windows::Win32::UI::Shell::{
    ExtractIconExW, IShellItem, IShellItemImageFactory, SHCreateItemFromParsingName,
    SHGetFileInfoW, SHFILEINFOW, SHGFI_ICON, SHGFI_LARGEICON, SHGFI_SMALLICON,
    SHGFI_USEFILEATTRIBUTES, SIIGBF_BIGGERSIZEOK, SIIGBF_ICONONLY, SIIGBF_RESIZETOFIT,
};
use windows::Win32::UI::WindowsAndMessaging::{LoadIconW, HICON, IDI_APPLICATION};

struct ComGuard {
    should_uninitialize: bool,
    _thread_affine: PhantomData<Rc<()>>,
}

impl ComGuard {
    fn new() -> Self {
        // SAFETY: [Category 8 - FFI boundary] `None` supplies COM's required null
        // reserved pointer, and the documented apartment flag is a valid value.
        let result = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) };
        Self {
            should_uninitialize: result.is_ok(),
            _thread_affine: PhantomData,
        }
    }
}

impl Drop for ComGuard {
    fn drop(&mut self) {
        if self.should_uninitialize {
            // SAFETY: [Category 13 - library contract] a successful `CoInitializeEx`
            // is balanced exactly once on the same thread; the `Rc` phantom makes
            // this guard neither `Send` nor `Sync`, preventing cross-thread transfer.
            unsafe { CoUninitialize() };
        }
    }
}

fn to_wide(value: &OsStr) -> Vec<u16> {
    value.encode_wide().chain(std::iter::once(0)).collect()
}

pub(super) fn shell_item_image_from_path(
    path: &Path,
    size: u32,
) -> Result<RgbaImage, ExtractError> {
    let wide = to_wide(path.as_os_str());
    shell_item_image_from_wide(&wide, size)
}

pub(super) fn shell_item_image_from_parsing_name(
    parsing_name: &str,
    size: u32,
) -> Result<RgbaImage, ExtractError> {
    let wide = to_wide(OsStr::new(parsing_name));
    shell_item_image_from_wide(&wide, size)
}

fn shell_item_image_from_wide(wide: &[u16], size: u32) -> Result<RgbaImage, ExtractError> {
    let _com = ComGuard::new();
    let extent =
        i32::try_from(size).map_err(|_| ExtractError::Api("icon size exceeds i32".into()))?;
    // SAFETY: [Category 8 - FFI boundary] `wide` is a live, NUL-terminated UTF-16
    // slice for the duration of the synchronous call; COM initialization was attempted.
    let item: IShellItem = unsafe { SHCreateItemFromParsingName(PCWSTR(wide.as_ptr()), None) }
        .map_err(|error| ExtractError::Api(format!("SHCreateItemFromParsingName: {error}")))?;
    let factory: IShellItemImageFactory = item
        .cast()
        .map_err(|error| ExtractError::Api(format!("IShellItemImageFactory: {error}")))?;
    // SAFETY: [Category 8 - FFI boundary] the COM interface is live, dimensions are
    // positive `i32` values, and the flags request a caller-owned icon bitmap.
    let handle = unsafe {
        factory.GetImage(
            SIZE {
                cx: extent,
                cy: extent,
            },
            SIIGBF_ICONONLY | SIIGBF_BIGGERSIZEOK | SIIGBF_RESIZETOFIT,
        )
    }
    .map_err(|error| ExtractError::Api(format!("GetImage: {error}")))?;
    if handle.is_invalid() {
        return Err(ExtractError::Api("GetImage returned invalid bitmap".into()));
    }
    // SAFETY: [Category 12 - invalid/double free] successful `GetImage` transfers
    // one valid HBITMAP to the caller; this is its sole owning wrapper.
    let bitmap: Owned<HBITMAP> = unsafe { Owned::new(handle) };
    gdi::hbitmap_to_rgba(*bitmap, size).map_err(ExtractError::Api)
}

pub(super) fn sh_get_file_info_image(path: &Path, size: u32) -> Result<RgbaImage, ExtractError> {
    let wide = to_wide(path.as_os_str());
    let mut info = SHFILEINFOW::default();
    let info_size = u32::try_from(std::mem::size_of::<SHFILEINFOW>())
        .map_err(|_| ExtractError::Api("SHFILEINFOW size exceeds u32".into()))?;
    let flags = if size <= 24 {
        SHGFI_ICON | SHGFI_SMALLICON
    } else {
        SHGFI_ICON | SHGFI_LARGEICON
    };
    // SAFETY: [Category 8 - FFI boundary] `wide` and `info` remain live and
    // correctly sized; SHGFI_ICON writes a caller-owned HICON on success.
    let result = unsafe {
        SHGetFileInfoW(
            PCWSTR(wide.as_ptr()),
            Default::default(),
            Some(&mut info),
            info_size,
            flags,
        )
    };
    if result == 0 || info.hIcon.is_invalid() {
        info = SHFILEINFOW::default();
        // SAFETY: [Category 8 - FFI boundary] the same live buffers are used;
        // FILE_ATTRIBUTE_NORMAL is valid with SHGFI_USEFILEATTRIBUTES.
        let retry = unsafe {
            SHGetFileInfoW(
                PCWSTR(wide.as_ptr()),
                windows::Win32::Storage::FileSystem::FILE_ATTRIBUTE_NORMAL,
                Some(&mut info),
                info_size,
                flags | SHGFI_USEFILEATTRIBUTES,
            )
        };
        if retry == 0 || info.hIcon.is_invalid() {
            return Err(ExtractError::Api("SHGetFileInfoW failed".into()));
        }
    }
    // SAFETY: [Category 12 - invalid/double free] successful SHGFI_ICON returns
    // one caller-owned HICON, and no other owner is constructed for this handle.
    let icon: Owned<HICON> = unsafe { Owned::new(info.hIcon) };
    gdi::hicon_to_rgba(*icon, size)
}

pub(super) fn extract_icon_ex_image(path: &Path, size: u32) -> Result<RgbaImage, ExtractError> {
    let wide = to_wide(path.as_os_str());
    let mut large_handle = HICON::default();
    let mut small_handle = HICON::default();
    // SAFETY: [Category 8 - FFI boundary] the path is live and NUL-terminated;
    // both out-pointers reference initialized HICON slots and capacity is one.
    let count = unsafe {
        ExtractIconExW(
            PCWSTR(wide.as_ptr()),
            0,
            Some(&mut large_handle),
            Some(&mut small_handle),
            1,
        )
    };
    let handles_alias = !large_handle.is_invalid()
        && !small_handle.is_invalid()
        && large_handle.0 == small_handle.0;
    let large = if large_handle.is_invalid() {
        None
    } else {
        // SAFETY: [Category 12 - invalid/double free] ExtractIconExW transfers
        // ownership of each distinct valid returned icon to the caller.
        Some(unsafe { Owned::new(large_handle) })
    };
    let small = if small_handle.is_invalid() || handles_alias {
        None
    } else {
        // SAFETY: [Category 12 - invalid/double free] this handle is valid and
        // distinct from the large icon, so it has exactly one owning wrapper.
        Some(unsafe { Owned::new(small_handle) })
    };
    if count == 0 {
        return Err(ExtractError::Api("ExtractIconExW returned 0".into()));
    }
    if count == u32::MAX {
        return Err(ExtractError::Api("ExtractIconExW failed".into()));
    }
    let icon = large
        .as_deref()
        .or(small.as_deref())
        .copied()
        .ok_or_else(|| ExtractError::Api("ExtractIconExW empty icons".into()))?;
    gdi::hicon_to_rgba(icon, size)
}

pub(super) fn package_path_by_full_name(full_name: &str) -> Option<PathBuf> {
    use windows::Win32::Storage::Packaging::Appx::GetPackagePathByFullName;

    let wide = to_wide(OsStr::new(full_name));
    let mut length = 0_u32;
    // SAFETY: [Category 8 - FFI boundary] the package name is live and
    // NUL-terminated; a null output buffer is the documented sizing query.
    let _ = unsafe { GetPackagePathByFullName(PCWSTR(wide.as_ptr()), &mut length, PWSTR::null()) };
    if length == 0 {
        return None;
    }
    let capacity = usize::try_from(length).ok()?;
    let mut buffer = vec![0_u16; capacity];
    // SAFETY: [Category 8 - FFI boundary] `buffer` has exactly the capacity
    // advertised through `length` and cannot reallocate during this call.
    let result = unsafe {
        GetPackagePathByFullName(
            PCWSTR(wide.as_ptr()),
            &mut length,
            PWSTR(buffer.as_mut_ptr()),
        )
    };
    if result.is_err() {
        return None;
    }
    let used = usize::try_from(length).ok()?;
    if used > buffer.len() {
        return None;
    }
    buffer.truncate(used);
    let path = String::from_utf16_lossy(&buffer)
        .trim_end_matches('\0')
        .to_string();
    (!path.is_empty()).then(|| PathBuf::from(path))
}

#[allow(dead_code)]
pub(super) fn default_application_icon(size: u32) -> Result<RgbaImage, ExtractError> {
    // SAFETY: [Category 13 - library contract] `None` plus IDI_APPLICATION
    // requests a checked shared system icon that must not be destroyed.
    let icon = unsafe { LoadIconW(None, IDI_APPLICATION) }
        .map_err(|error| ExtractError::Api(format!("LoadIconW: {error}")))?;
    gdi::hicon_to_rgba(icon, size)
}
