mod audio;
mod config;
mod hotkey;
mod input;
mod login_item;
mod pack;
mod permissions;
mod press_tracker;
mod ui;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;

use config::Config;
use press_tracker::PressTracker;
use ui::{AppShared, KeyclackApp};

fn main() -> eframe::Result<()> {
    env_logger::init();

    let mut config = config::load().unwrap_or_else(|e| {
        log::warn!("failed to load config: {e}");
        Config::default()
    });

    let audio = audio::AudioEngine::new().expect("no audio output device");
    let packs = pack::load_all(audio.sample_rate()).expect("no sound packs found");

    let selected_index = packs
        .iter()
        .position(|p| p.id == config.pack_id)
        .unwrap_or_else(|| {
            log::warn!(
                "configured pack '{}' not found, falling back to first pack",
                config.pack_id
            );
            0
        });
    config.pack_id = packs[selected_index].id.clone();

    audio.send(audio::AudioCommand::SetPack(packs[selected_index].samples.clone()));
    audio.send(audio::AudioCommand::SetVolume(config.volume));

    let shared = Arc::new(AppShared {
        muted: AtomicBool::new(config.muted),
    });

    if !permissions::is_accessibility_trusted() {
        permissions::request_accessibility_trust();
    }

    let (key_tx, key_rx) = crossbeam_channel::bounded::<input::KeyEvent>(256);
    let mut listener = input::platform_listener();
    if let Err(e) = listener.start(key_tx) {
        log::error!("failed to start key listener: {e}");
    }

    let dispatch_audio_tx = audio.sender_clone();
    let dispatch_shared = shared.clone();
    std::thread::spawn(move || run_dispatch(key_rx, dispatch_audio_tx, dispatch_shared));

    let hotkey = match hotkey::MuteHotkey::register() {
        Ok(hk) => Some(hk),
        Err(e) => {
            log::warn!("hotkey registration failed: {e}");
            None
        }
    };

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([380.0, 320.0])
            .with_resizable(false),
        ..Default::default()
    };

    let accessibility_ok = permissions::is_accessibility_trusted();

    eframe::run_native(
        "keyclack",
        options,
        Box::new(move |_cc| {
            Ok(Box::new(KeyclackApp {
                config,
                packs,
                audio,
                hotkey,
                shared,
                status: None,
                accessibility_ok,
                last_permission_check: Instant::now(),
            }))
        }),
    )
}

fn run_dispatch(
    key_rx: crossbeam_channel::Receiver<input::KeyEvent>,
    audio_tx: crossbeam_channel::Sender<audio::AudioCommand>,
    shared: Arc<AppShared>,
) {
    let mut tracker = PressTracker::new();
    loop {
        let event = match key_rx.recv() {
            Ok(e) => e,
            Err(_) => {
                log::info!("key event channel disconnected, dispatch thread exiting");
                return;
            }
        };

        match event {
            input::KeyEvent::Up(code) => {
                tracker.on_up(code);
            }
            input::KeyEvent::Down(code) => {
                if !tracker.on_down(code) {
                    continue;
                }
                if shared.muted.load(Ordering::Relaxed) {
                    continue;
                }
                let _ = audio_tx.try_send(audio::AudioCommand::Play);
            }
        }
    }
}
