//! Windows: Discord's IPC endpoint is a named pipe.
//!
//! A pipe opened as a plain file blocks forever on a read. [`PipeStream`] looks
//! with `PeekNamedPipe` first, which gives reads a timeout and gives the
//! liveness check a way to look without blocking.

use std::ffi::c_void;
use std::fs::File;
use std::io::{self, Read, Write};
use std::os::windows::io::AsRawHandle;
use std::path::Path;
use std::ptr::null_mut;
use std::time::{Duration, Instant};

#[link(name = "kernel32")]
extern "system" {
    fn PeekNamedPipe(
        pipe: *mut c_void,
        buffer: *mut u8,
        size: u32,
        read: *mut u32,
        available: *mut u32,
        left: *mut u32,
    ) -> i32;
}

/// All pipe instances are taken by other clients right now.
const ERROR_PIPE_BUSY: i32 = 231;

/// How often a waiting read looks at the pipe.
const TICK: Duration = Duration::from_millis(2);

/// Bytes waiting to be read. Fails once the other end is gone.
pub(crate) fn available(file: &File) -> io::Result<u32> {
    let mut waiting = 0u32;
    // SAFETY: the handle is valid for the lifetime of `file`; only the
    // "bytes available" output is requested and it points to a live u32.
    let ok = unsafe {
        PeekNamedPipe(
            file.as_raw_handle(),
            null_mut(),
            0,
            null_mut(),
            &mut waiting,
            null_mut(),
        )
    };
    if ok == 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(waiting)
    }
}

/// Client end of a named pipe with a read timeout.
pub struct PipeStream {
    file: File,
    timeout: Duration,
}

impl PipeStream {
    pub fn open(path: &Path, timeout: Duration) -> io::Result<PipeStream> {
        let mut tries = 0;
        loop {
            match std::fs::OpenOptions::new()
                .read(true)
                .write(true)
                .open(path)
            {
                Ok(file) => return Ok(PipeStream { file, timeout }),
                // Discord opens a new instance right after taking a client; ask again.
                Err(e) if e.raw_os_error() == Some(ERROR_PIPE_BUSY) && tries < 10 => {
                    tries += 1;
                    std::thread::sleep(Duration::from_millis(20));
                }
                Err(e) => return Err(e),
            }
        }
    }

    /// Bytes waiting to be read; an error means the pipe is closed.
    pub fn available(&self) -> io::Result<u32> {
        available(&self.file)
    }
}

impl Read for PipeStream {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let deadline = Instant::now() + self.timeout;
        loop {
            match available(&self.file) {
                Ok(0) => {}
                Ok(_) => return self.file.read(buf),
                // The other end closed the pipe: end of stream.
                Err(_) => return Ok(0),
            }
            if Instant::now() >= deadline {
                return Err(io::ErrorKind::TimedOut.into());
            }
            std::thread::sleep(TICK);
        }
    }
}

impl Write for PipeStream {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.file.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        // Flushing a pipe waits until the reader took everything; not wanted.
        Ok(())
    }
}

/// Server end, for the fake Discord in tests.
#[cfg(test)]
pub(crate) mod server {
    use super::*;
    use std::os::windows::ffi::OsStrExt;
    use std::os::windows::io::FromRawHandle;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Arc, Mutex};

    #[link(name = "kernel32")]
    extern "system" {
        fn CreateNamedPipeW(
            name: *const u16,
            open_mode: u32,
            pipe_mode: u32,
            max_instances: u32,
            out_buffer: u32,
            in_buffer: u32,
            default_timeout: u32,
            security: *mut c_void,
        ) -> *mut c_void;
        fn ConnectNamedPipe(pipe: *mut c_void, overlapped: *mut c_void) -> i32;
        fn DisconnectNamedPipe(pipe: *mut c_void) -> i32;
    }

    const PIPE_ACCESS_DUPLEX: u32 = 3;
    const PIPE_UNLIMITED_INSTANCES: u32 = 255;
    const ERROR_PIPE_CONNECTED: i32 = 535;

    fn new_instance(name: &[u16]) -> io::Result<File> {
        // SAFETY: `name` is a NUL-terminated UTF-16 string that outlives the call.
        let handle = unsafe {
            CreateNamedPipeW(
                name.as_ptr(),
                PIPE_ACCESS_DUPLEX,
                0, // byte stream, blocking
                PIPE_UNLIMITED_INSTANCES,
                65536,
                65536,
                0,
                null_mut(),
            )
        };
        if handle as isize == -1 {
            return Err(io::Error::last_os_error());
        }
        // SAFETY: a fresh handle that nothing else owns.
        Ok(unsafe { File::from_raw_handle(handle as _) })
    }

    pub struct Listener {
        name: Vec<u16>,
        /// The instance the next client will get; always present so a client never
        /// finds the pipe missing.
        waiting: Mutex<Option<File>>,
    }

    impl Listener {
        pub fn bind(path: &Path) -> io::Result<Listener> {
            let name: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
            let first = new_instance(&name)?;
            Ok(Listener {
                name,
                waiting: Mutex::new(Some(first)),
            })
        }

        pub fn accept(&self) -> io::Result<Conn> {
            let file = self.waiting.lock().unwrap().take().expect("an instance");
            // SAFETY: the handle belongs to `file`; no overlapped structure is used.
            let ok = unsafe { ConnectNamedPipe(file.as_raw_handle(), null_mut()) };
            let err = io::Error::last_os_error();
            *self.waiting.lock().unwrap() = Some(new_instance(&self.name)?);
            if ok == 0 && err.raw_os_error() != Some(ERROR_PIPE_CONNECTED) {
                return Err(err);
            }
            Ok(Conn {
                file: Arc::new(file),
                closed: Arc::new(AtomicBool::new(false)),
            })
        }
    }

    /// One accepted client. Clones share the pipe; `close` makes every clone fail.
    #[derive(Clone)]
    pub struct Conn {
        file: Arc<File>,
        closed: Arc<AtomicBool>,
    }

    impl Conn {
        /// Hang up: the client sees the pipe as closed right away.
        pub fn close(&self) {
            self.closed.store(true, Ordering::SeqCst);
            // SAFETY: the handle belongs to `self.file`, which is alive.
            unsafe { DisconnectNamedPipe(self.file.as_raw_handle()) };
        }
    }

    impl Read for Conn {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            loop {
                if self.closed.load(Ordering::SeqCst) {
                    return Err(io::ErrorKind::BrokenPipe.into());
                }
                match available(&self.file) {
                    Ok(0) => std::thread::sleep(TICK),
                    Ok(_) => return (&*self.file).read(buf),
                    Err(_) => return Ok(0),
                }
            }
        }
    }

    impl Write for Conn {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            if self.closed.load(Ordering::SeqCst) {
                return Err(io::ErrorKind::BrokenPipe.into());
            }
            (&*self.file).write(buf)
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
}
