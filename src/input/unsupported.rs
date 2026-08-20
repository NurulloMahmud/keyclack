use super::{InputError, KeyEvent, KeyListener};

/// Stub listener for platforms with no v1 implementation. Only used on non-macOS builds.
#[allow(dead_code)]
pub struct UnsupportedListener;

impl KeyListener for UnsupportedListener {
    fn start(&mut self, _sink: crossbeam_channel::Sender<KeyEvent>) -> Result<(), InputError> {
        // SPEC-NOTE: v1 is macOS-only by decision. To add Windows, implement this trait using
        // SetWindowsHookExW(WH_KEYBOARD_LL) on a dedicated thread with a GetMessage pump.
        // To add Linux/X11, use XRecord. Wayland requires reading /dev/input/event* via evdev,
        // which needs the user to be in the `input` group. No other module should need changes.
        Err(InputError::UnsupportedPlatform)
    }
}
