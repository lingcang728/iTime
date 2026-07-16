use std::marker::PhantomData;
use std::rc::Rc;
use windows::core::Owned;
use windows::Win32::Foundation::HWND;
use windows::Win32::Graphics::Gdi::{
    CreateCompatibleDC, DeleteDC, GetDC, ReleaseDC, SelectObject, HBITMAP, HDC, HGDIOBJ,
};

pub(super) struct ScreenDc {
    window: HWND,
    handle: HDC,
    _thread_affine: PhantomData<Rc<()>>,
}

impl ScreenDc {
    pub(super) fn acquire() -> Result<Self, String> {
        let window = HWND::default();
        // SAFETY: [Category 8 - FFI boundary] a null HWND is the documented
        // request for the screen DC, and the returned handle is checked.
        let handle = unsafe { GetDC(window) };
        if handle.is_invalid() {
            return Err("GetDC failed".into());
        }
        Ok(Self {
            window,
            handle,
            _thread_affine: PhantomData,
        })
    }

    pub(super) fn handle(&self) -> HDC {
        self.handle
    }
}

impl Drop for ScreenDc {
    fn drop(&mut self) {
        // SAFETY: [Category 13 - library contract] this is the same HWND/HDC
        // pair returned by GetDC and the guard cannot cross threads.
        let _ = unsafe { ReleaseDC(self.window, self.handle) };
    }
}

pub(super) struct MemoryDc {
    handle: HDC,
    _thread_affine: PhantomData<Rc<()>>,
}

impl MemoryDc {
    pub(super) fn compatible_with(source: HDC) -> Result<Self, String> {
        // SAFETY: [Category 8 - FFI boundary] `source` is a checked live screen
        // DC; the returned compatible DC is checked before use.
        let handle = unsafe { CreateCompatibleDC(source) };
        if handle.is_invalid() {
            return Err("CreateCompatibleDC failed".into());
        }
        Ok(Self {
            handle,
            _thread_affine: PhantomData,
        })
    }

    pub(super) fn handle(&self) -> HDC {
        self.handle
    }
}

impl Drop for MemoryDc {
    fn drop(&mut self) {
        // SAFETY: [Category 13 - library contract] this DC came from
        // CreateCompatibleDC, remains owned here, and is deleted on its thread.
        let _ = unsafe { DeleteDC(self.handle) };
    }
}

pub(super) struct BitmapSelection<'a> {
    dc: &'a MemoryDc,
    _bitmap: &'a Owned<HBITMAP>,
    previous: HGDIOBJ,
    restored: bool,
}

impl<'a> BitmapSelection<'a> {
    pub(super) fn select(dc: &'a MemoryDc, bitmap: &'a Owned<HBITMAP>) -> Result<Self, String> {
        // SAFETY: [Category 13 - library contract] both handles are checked and
        // live, and the guard borrows them until the previous object is restored.
        let previous = unsafe { SelectObject(dc.handle(), HGDIOBJ((**bitmap).0)) };
        if previous.is_invalid() {
            return Err("SelectObject failed".into());
        }
        Ok(Self {
            dc,
            _bitmap: bitmap,
            previous,
            restored: false,
        })
    }

    pub(super) fn restore(mut self) -> Result<(), String> {
        // SAFETY: [Category 13 - library contract] `previous` is the exact GDI
        // object returned by SelectObject for this still-live memory DC.
        let replaced = unsafe { SelectObject(self.dc.handle(), self.previous) };
        if replaced.is_invalid() {
            return Err("SelectObject restore failed".into());
        }
        self.restored = true;
        Ok(())
    }
}

impl Drop for BitmapSelection<'_> {
    fn drop(&mut self) {
        if !self.restored {
            // SAFETY: [Category 13 - library contract] this fallback uses the
            // saved object and live borrowed DC; return failure only causes cleanup.
            let _ = unsafe { SelectObject(self.dc.handle(), self.previous) };
        }
    }
}
