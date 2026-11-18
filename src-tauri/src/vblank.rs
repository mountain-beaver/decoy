//! Linux: let the window follow the monitor's real refresh rate.
//!
//! WebKitGTK paces its frames with `drmWaitVBlank`. Some drivers refuse that
//! call (NVIDIA's proprietary one does), and WebKitGTK then falls back to a
//! fixed 60 Hz timer, which looks choppy on a 144 or 165 Hz display.
//!
//! This executable exports its own `drmWaitVBlank`. The dynamic linker looks
//! symbols up in the executable first, so WebKitGTK calls this one. It forwards
//! to the real function, and only if the very first call fails it takes over
//! and paces frames itself from the refresh rate of the display mode.
//! `DECOY_VBLANK=off` turns the takeover off.

use std::ffi::{c_char, c_int, c_long, c_ulong, c_void, CStr};
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Request {
    kind: u32,
    sequence: u32,
    signal: c_ulong,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Reply {
    kind: u32,
    sequence: u32,
    tval_sec: c_long,
    tval_usec: c_long,
}

/// `drmVBlank` from xf86drm.h.
#[repr(C)]
pub union DrmVBlank {
    request: Request,
    reply: Reply,
}

/// Leading fields of `drmModeRes`; the rest is not needed.
#[repr(C)]
struct ModeRes {
    count_fbs: c_int,
    fbs: *mut u32,
    count_crtcs: c_int,
    crtcs: *mut u32,
}

#[repr(C)]
struct ModeInfo {
    clock: u32,
    hdisplay: u16,
    hsync_start: u16,
    hsync_end: u16,
    htotal: u16,
    hskew: u16,
    vdisplay: u16,
    vsync_start: u16,
    vsync_end: u16,
    vtotal: u16,
    vscan: u16,
    vrefresh: u32,
    flags: u32,
    kind: u32,
    name: [c_char; 32],
}

#[repr(C)]
struct ModeCrtc {
    crtc_id: u32,
    buffer_id: u32,
    x: u32,
    y: u32,
    width: u32,
    height: u32,
    mode_valid: c_int,
    mode: ModeInfo,
    gamma_size: c_int,
}

const VBLANK_RELATIVE: u32 = 0x1;
const VBLANK_HIGH_CRTC_MASK: u32 = 0x3e;
const VBLANK_HIGH_CRTC_SHIFT: u32 = 1;
const VBLANK_SECONDARY: u32 = 0x2000_0000;

const UNDECIDED: u8 = 0;
const REAL: u8 = 1;
const PACED: u8 = 2;
static MODE: AtomicU8 = AtomicU8::new(UNDECIDED);

/// Refresh rate per CRTC index, with the time it was read.
static RATES: Mutex<[Option<(f64, Instant)>; 8]> = Mutex::new([None; 8]);

unsafe fn sym(handle: *mut c_void, name: &CStr) -> *mut c_void {
    libc::dlsym(handle, name.as_ptr())
}

/// The mode's refresh rate in Hz for the CRTC at `index`, cached for a few seconds.
unsafe fn refresh_rate(fd: c_int, index: usize) -> f64 {
    const FALLBACK: f64 = 60.0;
    if let Ok(rates) = RATES.lock() {
        if let Some(Some((hz, at))) = rates.get(index) {
            if at.elapsed() < Duration::from_secs(3) {
                return *hz;
            }
        }
    }
    type GetRes = unsafe extern "C" fn(c_int) -> *mut ModeRes;
    type FreeRes = unsafe extern "C" fn(*mut ModeRes);
    type GetCrtc = unsafe extern "C" fn(c_int, u32) -> *mut ModeCrtc;
    type FreeCrtc = unsafe extern "C" fn(*mut ModeCrtc);
    let (get_res, free_res, get_crtc, free_crtc) = (
        sym(libc::RTLD_DEFAULT, c"drmModeGetResources"),
        sym(libc::RTLD_DEFAULT, c"drmModeFreeResources"),
        sym(libc::RTLD_DEFAULT, c"drmModeGetCrtc"),
        sym(libc::RTLD_DEFAULT, c"drmModeFreeCrtc"),
    );
    if get_res.is_null() || free_res.is_null() || get_crtc.is_null() || free_crtc.is_null() {
        return FALLBACK;
    }
    let get_res: GetRes = std::mem::transmute(get_res);
    let free_res: FreeRes = std::mem::transmute(free_res);
    let get_crtc: GetCrtc = std::mem::transmute(get_crtc);
    let free_crtc: FreeCrtc = std::mem::transmute(free_crtc);

    let mut hz = FALLBACK;
    let res = get_res(fd);
    if !res.is_null() {
        if index < (*res).count_crtcs.max(0) as usize && !(*res).crtcs.is_null() {
            let crtc = get_crtc(fd, *(*res).crtcs.add(index));
            if !crtc.is_null() {
                let m = &(*crtc).mode;
                let total = m.htotal as f64 * m.vtotal as f64;
                if (*crtc).mode_valid != 0 && total > 0.0 && m.clock > 0 {
                    hz = (m.clock as f64 * 1000.0 / total).clamp(24.0, 1000.0);
                }
                free_crtc(crtc);
            }
        }
        free_res(res);
    }
    if let Ok(mut rates) = RATES.lock() {
        if let Some(slot) = rates.get_mut(index) {
            *slot = Some((hz, Instant::now()));
        }
    }
    hz
}

// The casts are no-ops on 64-bit targets and needed on 32-bit ones.
#[allow(clippy::unnecessary_cast)]
fn monotonic_ns() -> i64 {
    let mut ts = libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    unsafe { libc::clock_gettime(libc::CLOCK_MONOTONIC, &mut ts) };
    ts.tv_sec as i64 * 1_000_000_000 + ts.tv_nsec as i64
}

fn sleep_until(ns: i64) {
    let ts = libc::timespec {
        tv_sec: (ns / 1_000_000_000) as libc::time_t,
        tv_nsec: (ns % 1_000_000_000) as c_long,
    };
    unsafe {
        while libc::clock_nanosleep(
            libc::CLOCK_MONOTONIC,
            libc::TIMER_ABSTIME,
            &ts,
            std::ptr::null_mut(),
        ) == libc::EINTR
        {}
    }
}

/// Stand in for the vertical blank: wait for the next tick of the mode's refresh rate.
unsafe fn paced(fd: c_int, req: Request, vbl: *mut DrmVBlank) -> c_int {
    let index = if req.kind & VBLANK_HIGH_CRTC_MASK != 0 {
        ((req.kind & VBLANK_HIGH_CRTC_MASK) >> VBLANK_HIGH_CRTC_SHIFT) as usize
    } else if req.kind & VBLANK_SECONDARY != 0 {
        1
    } else {
        0
    };
    let period = (1e9 / refresh_rate(fd, index)) as i64;
    let now = monotonic_ns();
    let current = now / period;
    let target = if req.kind & VBLANK_RELATIVE != 0 {
        current + req.sequence as i64
    } else {
        (req.sequence as i64).max(current)
    };
    let mut at = now;
    if target > current {
        at = target * period;
        sleep_until(at);
    }
    (*vbl).reply = Reply {
        kind: req.kind,
        sequence: target as u32,
        tval_sec: (at / 1_000_000_000) as c_long,
        tval_usec: ((at % 1_000_000_000) / 1000) as c_long,
    };
    0
}

/// Replaces libdrm's function for this process. See the module comment.
///
/// # Safety
/// Called by WebKitGTK with the arguments of libdrm's `drmWaitVBlank`.
#[no_mangle]
pub unsafe extern "C" fn drmWaitVBlank(fd: c_int, vbl: *mut DrmVBlank) -> c_int {
    type Real = unsafe extern "C" fn(c_int, *mut DrmVBlank) -> c_int;
    if vbl.is_null() {
        return -libc::EINVAL;
    }
    let req = (*vbl).request;
    let mode = MODE.load(Ordering::Relaxed);
    if mode == PACED {
        return paced(fd, req, vbl);
    }
    let real = sym(libc::RTLD_NEXT, c"drmWaitVBlank");
    let result = if real.is_null() {
        -libc::ENOSYS
    } else {
        std::mem::transmute::<*mut c_void, Real>(real)(fd, vbl)
    };
    if mode == UNDECIDED {
        // The first call decides: a driver that answers it keeps being used as is.
        let takeover = result != 0 && std::env::var_os("DECOY_VBLANK").is_none_or(|v| v != "off");
        MODE.store(if takeover { PACED } else { REAL }, Ordering::Relaxed);
        if takeover {
            (*vbl).request = req;
            return paced(fd, req, vbl);
        }
    }
    result
}

/// Call once at startup, before GTK and WebKit are initialised.
pub fn init() {
    // Keeps the exported function in the binary.
    std::hint::black_box(drmWaitVBlank as *const ());

    // With NVIDIA's proprietary driver WebKitGTK's dmabuf renderer presents at
    // 60 Hz whatever the display does (measured: scrolling at 60 fps on a 165 Hz
    // monitor, 165 fps without it) and is known for blank windows. Use the other
    // renderer there unless the user decided otherwise.
    const VAR: &str = "WEBKIT_DISABLE_DMABUF_RENDERER";
    if std::env::var_os(VAR).is_none()
        && std::path::Path::new("/proc/driver/nvidia/version").exists()
    {
        std::env::set_var(VAR, "1");
    }
}
