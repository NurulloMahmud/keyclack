#[cfg(target_os = "macos")]
pub mod macos;
pub mod unsupported;

/// A raw, platform-native key transition. `u16` is the platform virtual key code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyEvent {
    Down(u16),
    Up(u16),
}

/// Errors that can occur while starting a platform key listener.
#[derive(Debug)]
pub enum InputError {
    /// macOS Accessibility permission has not been granted.
    PermissionDenied,
    /// CGEventTapCreate returned null.
    TapCreationFailed,
    /// This platform has no implementation in v1. Only reachable on non-macOS builds.
    #[allow(dead_code)]
    UnsupportedPlatform,
}

impl std::fmt::Display for InputError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InputError::PermissionDenied => write!(f, "accessibility permission not granted"),
            InputError::TapCreationFailed => write!(f, "failed to create event tap"),
            InputError::UnsupportedPlatform => write!(f, "this platform is not supported in v1"),
        }
    }
}

impl std::error::Error for InputError {}

/// A background source of global key events.
pub trait KeyListener: Send {
    /// Spawn the listener. Returns once the listener is running. Events are pushed to `sink`.
    /// The listener runs until the process exits; there is no stop method in v1.
    fn start(&mut self, sink: crossbeam_channel::Sender<KeyEvent>) -> Result<(), InputError>;
}

/// Returns the listener for the current platform.
pub fn platform_listener() -> Box<dyn KeyListener> {
    #[cfg(target_os = "macos")]
    {
        Box::new(crate::input::macos::MacosListener)
    }
    #[cfg(not(target_os = "macos"))]
    {
        Box::new(crate::input::unsupported::UnsupportedListener)
    }
}
