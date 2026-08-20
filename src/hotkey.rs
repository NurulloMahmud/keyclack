use global_hotkey::hotkey::{Code, HotKey, Modifiers};
use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState};

/// The fixed global mute hotkey: Control + Option + M. Not configurable in v1.
pub struct MuteHotkey {
    _manager: GlobalHotKeyManager,
    _hotkey: HotKey,
}

/// Errors that can occur while registering the global mute hotkey.
#[derive(Debug)]
pub enum HotkeyError {
    RegistrationFailed(String),
}

impl std::fmt::Display for HotkeyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HotkeyError::RegistrationFailed(s) => write!(f, "hotkey registration failed: {s}"),
        }
    }
}

impl std::error::Error for HotkeyError {}

impl MuteHotkey {
    /// Registers Ctrl+Alt+M with the OS. The manager must be kept alive for the hotkey to work.
    pub fn register() -> Result<Self, HotkeyError> {
        let manager =
            GlobalHotKeyManager::new().map_err(|e| HotkeyError::RegistrationFailed(e.to_string()))?;
        let hotkey = HotKey::new(Some(Modifiers::CONTROL | Modifiers::ALT), Code::KeyM);
        manager
            .register(hotkey)
            .map_err(|e| HotkeyError::RegistrationFailed(e.to_string()))?;

        Ok(Self {
            _manager: manager,
            _hotkey: hotkey,
        })
    }

    /// Drains pending hotkey events. Returns true if the mute toggle fired at least once.
    /// Must be called from the UI thread on every frame.
    pub fn poll_triggered(&self) -> bool {
        let mut fired = false;
        while let Ok(event) = GlobalHotKeyEvent::receiver().try_recv() {
            if event.state == HotKeyState::Pressed {
                fired = true;
            }
        }
        fired
    }
}
