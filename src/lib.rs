//! Gracefully shutdown.
//!
//! (Unix) Async signals are tricky to handle properly. This utility let you
//! block the main thread and execute arbitrary (thread-safe) code to shutdown
//! the process gracefully.
//!
//! # Usage
//!
//! 1. Initialize a [SignalGuard](struct.SignalGuard.html) before creating any
//!    additional threads.
//! 2. The [SignalGuard](struct.SignalGuard.html) will block necessary signals
//!    (`SIGINT`, `SIGQUIT` and `SIGTERM` on *nix, `Ctrl+C` and `Ctrl+Break` on
//!    Windows) during initialization.
//! 3. Spawn new threads to do the real work.
//! 4. Register a handle to properly shutdown the application.
//! 5. The main thread will be blocked until a signal is received.
//! 6. The handler will run in the main thread.
//! 7. On Windows the process will terminate after the handler returns (and
//!    potentially any libc `atexit` handlers).
//!
//! # Example
//!
//! ```no_run
//! extern crate graceful;
//!
//! use std::sync::atomic::{ATOMIC_BOOL_INIT, AtomicBool, Ordering};
//! use std::time::Duration;
//! use std::thread;
//!
//! use graceful::SignalGuard;
//!
//! static STOP: AtomicBool = ATOMIC_BOOL_INIT;
//!
//! fn main() {
//!     let signal_guard = SignalGuard::new();
//!
//! 	let handle = thread::spawn(|| {
//!         println!("Worker thread started. Type Ctrl+C to stop.");
//!         while !STOP.load(Ordering::Acquire) {
//!             println!("working...");
//!             thread::sleep(Duration::from_millis(500));
//!         }
//!         println!("Bye.");
//!     });
//!
//! 	signal_guard.at_exit(move |sig| {
//!         println!("Signal {} received.", sig);
//!         STOP.store(true, Ordering::Release);
//!         handle.join().unwrap();
//!     });
//! }
//! ```
//!

#[cfg(unix)]
mod platform {
    extern crate nix;
    use self::nix::sys::signal::{SigSet, SIGINT, SIGQUIT, SIGTERM};

    pub struct SignalGuard(SigSet);

    impl SignalGuard {
        /// Block necessary signals (`SIGINT`, `SIGQUIT` and `SIGTERM` on *nix,
        /// `Ctrl+C` and `Ctrl+Break` on Windows).
        ///
        /// New threads should be spawned after this.
        pub fn new() -> SignalGuard {
            let mut mask = SigSet::empty();
            SignalGuard::init(&mut mask).unwrap();
            SignalGuard(mask)
        }

        fn init(mask: &mut SigSet) -> nix::Result<()> {
            mask.add(SIGINT);
            mask.add(SIGQUIT);
            mask.add(SIGTERM);
            mask.thread_block()
        }

        /// Block the running thread until a signal is received. Then the
        /// `handler` will be called in the main thread.
        ///
        /// Do not put any code after this.
        pub fn at_exit<F: FnOnce(usize)>(&self, handler: F) {
            let sig = self.0.wait().unwrap();
            handler(sig as usize);
        }
    }
}

#[cfg(windows)]
mod platform {
    extern crate winapi;
    extern crate kernel32;

    use std::sync::mpsc::{sync_channel, SyncSender, Receiver};

    use kernel32::SetConsoleCtrlHandler;
    use winapi::{BOOL, DWORD, TRUE};

    static CHAN: (SyncSender<DWORD>, Receiver<DWORD>) = sync_channel(0);

    unsafe extern "system" fn handler(event: DWORD) -> BOOL {
        CHAN.0.send(event);
        CHAN.0.send(0);
        FALSE
    }

    pub struct SignalGuard;

    impl SignalGuard {
        pub fn new() -> SignalGuard {
            unsafe { SetConsoleCtrlHandler(Some(handler), TRUE) };
            SignalGuard
        }

        pub fn at_exit<F: FnOnce(usize)>(&self, handler: F) {
            let event = CHAN.1.recv().unwrap();
            handler(event as usize);
            CHAN.1.recv().unwrap();
        }
    }
}

pub use platform::SignalGuard;
