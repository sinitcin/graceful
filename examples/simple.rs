extern crate graceful;

use std::sync::atomic::{ATOMIC_BOOL_INIT, AtomicBool, Ordering};
use std::time::Duration;
use std::thread;

use graceful::SignalGuard;

static STOP: AtomicBool = ATOMIC_BOOL_INIT;

fn main() {
    let signal_guard = SignalGuard::new();

    let handle = thread::spawn(|| {
        println!("Worker thread started. Type Ctrl+C to stop.");
        while !STOP.load(Ordering::Acquire) {
            println!("working...");
            thread::sleep(Duration::from_millis(500));
        }
        println!("Bye.");
    });

    signal_guard.at_exit(move |sig| {
        println!("Signal {} received.", sig);
        STOP.store(true, Ordering::Release);
        handle.join().unwrap();
    });
}
