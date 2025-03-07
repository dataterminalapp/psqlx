// spinner.rs
use std::io::{self, Write};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::thread;
use std::time::Duration;

/// A simple spinner animation that can be started and stopped
pub struct Spinner {
    running: Arc<AtomicBool>,
    handle: Option<thread::JoinHandle<()>>,
}

impl Spinner {
    pub fn start() -> Self {
        let running = Arc::new(AtomicBool::new(true));
        let running_clone = Arc::clone(&running);

        let handle = thread::spawn(move || {
            let spinner_chars = vec!['|', '/', '-', '\\'];
            let mut i = 0;

            while running_clone.load(Ordering::Relaxed) {
                print!("\r{}", spinner_chars[i % 4]);
                io::stdout().flush().unwrap();
                i += 1;
                thread::sleep(Duration::from_millis(100));
            }

            // Clear the spinner
            print!("\r \r");
            io::stdout().flush().unwrap();
        });

        Spinner {
            running,
            handle: Some(handle),
        }
    }

    /// Stops the spinner animation and waits for the thread to finish
    pub fn stop(&mut self) {
        self.running.store(false, Ordering::Relaxed);
        if let Some(handle) = self.handle.take() {
            handle.join().unwrap();
        }
    }
}

impl Drop for Spinner {
    fn drop(&mut self) {
        // Ensure the spinner is stopped if it hasn't been explicitly stopped
        if self.running.load(Ordering::Relaxed) {
            self.stop();
        }
    }
}

/// Creates a spinner that automatically stops when the provided function completes
pub fn with_spinner<F, T>(f: F) -> T
where
    F: FnOnce() -> T,
{
    let mut spinner = Spinner::start();
    let result = f();
    spinner.stop();
    result
}
