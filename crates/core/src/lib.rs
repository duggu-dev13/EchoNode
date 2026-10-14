pub mod config;
pub mod queue;
pub mod source;

// mpv control uses a Windows named pipe for now.
// A Unix-socket version will be added when we support Linux/macOS.
#[cfg(windows)]
pub mod player;
