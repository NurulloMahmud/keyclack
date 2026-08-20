use std::cell::Cell;
use std::rc::Rc;

use core_foundation::base::TCFType;
use core_foundation::mach_port::CFMachPortRef;
use core_foundation::runloop::{kCFRunLoopCommonModes, CFRunLoop};
use core_graphics::event::{
    CGEventTap, CGEventTapLocation, CGEventTapOptions, CGEventTapPlacement, CGEventType,
    EventField,
};

use super::{InputError, KeyEvent, KeyListener};

// SPEC-QUESTION: 5.8 step 1 says the callback should call `tap.enable()` directly, but the
// callback closure is moved into `CGEventTap::new` before `tap` exists, so it cannot borrow
// `tap` itself (self-referential). Instead the raw CFMachPortRef is stashed in a Cell right
// after the tap is constructed (and before the run loop starts, so no event can race it), and
// the callback re-enables the tap through that raw pointer via CGEventTapEnable directly.
#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    fn CGEventTapEnable(tap: CFMachPortRef, enable: bool);
}

/// Global key listener backed by a macOS CGEventTap.
pub struct MacosListener;

impl KeyListener for MacosListener {
    fn start(&mut self, sink: crossbeam_channel::Sender<KeyEvent>) -> Result<(), InputError> {
        if !crate::permissions::is_accessibility_trusted() {
            return Err(InputError::PermissionDenied);
        }

        std::thread::Builder::new()
            .name("keyclack-eventtap".into())
            .spawn(move || run_event_tap(sink))
            .map_err(|e| {
                log::error!("failed to spawn event tap thread: {e}");
                InputError::TapCreationFailed
            })?;

        Ok(())
    }
}

fn run_event_tap(sink: crossbeam_channel::Sender<KeyEvent>) {
    let prev_flags: Cell<u64> = Cell::new(0);
    let mach_port_cell: Rc<Cell<Option<CFMachPortRef>>> = Rc::new(Cell::new(None));
    let mach_port_for_cb = mach_port_cell.clone();

    let tap = CGEventTap::new(
        CGEventTapLocation::Session,
        CGEventTapPlacement::HeadInsertEventTap,
        CGEventTapOptions::ListenOnly,
        vec![
            CGEventType::KeyDown,
            CGEventType::KeyUp,
            CGEventType::FlagsChanged,
        ],
        move |_proxy, event_type, event| {
            match event_type {
                CGEventType::TapDisabledByTimeout | CGEventType::TapDisabledByUserInput => {
                    log::warn!("event tap disabled by system, re-enabling");
                    if let Some(port) = mach_port_for_cb.get() {
                        unsafe { CGEventTapEnable(port, true) };
                    }
                    return None;
                }
                _ => {}
            }

            let code = event.get_integer_value_field(EventField::KEYBOARD_EVENT_KEYCODE) as u16;

            match event_type {
                CGEventType::KeyDown => {
                    let _ = sink.try_send(KeyEvent::Down(code));
                }
                CGEventType::KeyUp => {
                    let _ = sink.try_send(KeyEvent::Up(code));
                }
                CGEventType::FlagsChanged => {
                    let flags = event.get_flags().bits();
                    let prev = prev_flags.get();
                    prev_flags.set(flags);
                    if flags.count_ones() > prev.count_ones() {
                        let _ = sink.try_send(KeyEvent::Down(code));
                    } else {
                        let _ = sink.try_send(KeyEvent::Up(code));
                    }
                }
                _ => {}
            }

            None
        },
    );

    let tap = match tap {
        Ok(t) => t,
        Err(_) => {
            log::error!("CGEventTapCreate failed (event tap could not be created)");
            return;
        }
    };

    mach_port_cell.set(Some(tap.mach_port.as_concrete_TypeRef()));

    unsafe {
        let loop_source = match tap.mach_port.create_runloop_source(0) {
            Ok(s) => s,
            Err(_) => {
                log::error!("failed to create run loop source for event tap");
                return;
            }
        };
        let current = CFRunLoop::get_current();
        current.add_source(&loop_source, kCFRunLoopCommonModes);
        tap.enable();
        CFRunLoop::run_current();
    }
}
