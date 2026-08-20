use core_foundation::base::TCFType;
use core_foundation::boolean::CFBoolean;
use core_foundation::dictionary::{CFDictionary, CFDictionaryRef};
use core_foundation::string::CFString;

#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    fn AXIsProcessTrusted() -> bool;
    fn AXIsProcessTrustedWithOptions(options: CFDictionaryRef) -> bool;
}

/// True if this process is trusted for macOS Accessibility (required for the event tap).
/// Calls AXIsProcessTrusted(). Does not prompt.
pub fn is_accessibility_trusted() -> bool {
    unsafe { AXIsProcessTrusted() }
}

/// Calls AXIsProcessTrustedWithOptions with kAXTrustedCheckOptionPrompt = true,
/// which shows the system's "grant access" dialog. Returns the current trust state.
pub fn request_accessibility_trust() -> bool {
    let key = CFString::new("AXTrustedCheckOptionPrompt");
    let value = CFBoolean::true_value();
    let dict = CFDictionary::from_CFType_pairs(&[(key.as_CFType(), value.as_CFType())]);
    unsafe { AXIsProcessTrustedWithOptions(dict.as_concrete_TypeRef()) }
}

/// Opens System Settings → Privacy & Security → Accessibility via `open` with the
/// x-apple.systempreferences URL. Errors are logged, not returned.
pub fn open_accessibility_settings() {
    if let Err(e) = std::process::Command::new("open")
        .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility")
        .spawn()
    {
        log::error!("failed to open accessibility settings: {e}");
    }
}
