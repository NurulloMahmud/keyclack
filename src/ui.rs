use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::audio::{AudioCommand, AudioEngine};
use crate::config::Config;
use crate::pack::Pack;
use crate::{config, login_item, permissions};

/// Shared state between the UI thread and the key-event thread.
pub struct AppShared {
    /// Read by the key-event thread on every key press.
    pub muted: AtomicBool,
}

/// The eframe application.
pub struct KeyclackApp {
    pub config: Config,
    pub packs: Vec<Pack>,
    pub audio: AudioEngine,
    pub hotkey: Option<crate::hotkey::MuteHotkey>,
    pub shared: Arc<AppShared>,
    /// Non-fatal message shown in a red bar at the bottom, cleared on next successful action.
    pub status: Option<String>,
    /// Re-checked once per second, not per frame.
    pub accessibility_ok: bool,
    pub last_permission_check: Instant,
}

impl KeyclackApp {
    fn save_config(&mut self) {
        if let Err(e) = config::save(&self.config) {
            log::warn!("failed to save config: {e}");
            self.status = Some(format!("Failed to save settings: {e}"));
        }
    }
}

impl eframe::App for KeyclackApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.request_repaint_after(Duration::from_millis(100));

        if self.hotkey.as_ref().is_some_and(|h| h.poll_triggered()) {
            self.config.muted = !self.config.muted;
            self.shared.muted.store(self.config.muted, Ordering::Relaxed);
            self.save_config();
        }

        if self.last_permission_check.elapsed() > Duration::from_secs(1) {
            self.accessibility_ok = permissions::is_accessibility_trusted();
            self.last_permission_check = Instant::now();
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("keyclack");

            if !self.accessibility_ok {
                ui.colored_label(
                    egui::Color32::RED,
                    "Accessibility permission not granted — no sounds will play.",
                );
                if ui.button("Open System Settings").clicked() {
                    permissions::open_accessibility_settings();
                }
                ui.small(
                    "Running via cargo run? Grant access to your terminal app, not to keyclack.",
                );
            }

            ui.separator();

            let mut selected_id = self.config.pack_id.clone();
            let selected_name = self
                .packs
                .iter()
                .find(|p| p.id == selected_id)
                .map(|p| p.name.clone())
                .unwrap_or_default();
            egui::ComboBox::from_label("Sound pack")
                .selected_text(selected_name)
                .show_ui(ui, |ui| {
                    for pack in &self.packs {
                        ui.selectable_value(&mut selected_id, pack.id.clone(), &pack.name);
                    }
                });
            if selected_id != self.config.pack_id {
                self.config.pack_id = selected_id.clone();
                if let Some(pack) = self.packs.iter().find(|p| p.id == selected_id) {
                    self.audio.send(AudioCommand::SetPack(pack.samples.clone()));
                }
                self.save_config();
            }

            let mut volume_percent = (self.config.volume * 100.0).round() as i32;
            let slider =
                egui::Slider::new(&mut volume_percent, 0..=100).suffix("%").text("Volume");
            if ui.add(slider).changed() {
                self.config.volume = volume_percent as f32 / 100.0;
                self.audio.send(AudioCommand::SetVolume(self.config.volume));
                self.save_config();
            }

            if ui.checkbox(&mut self.config.muted, "Muted (⌃⌥M)").changed() {
                self.shared.muted.store(self.config.muted, Ordering::Relaxed);
                self.save_config();
            }

            ui.separator();

            let previous = self.config.start_on_login;
            if ui.checkbox(&mut self.config.start_on_login, "Start on login").changed() {
                let result = if self.config.start_on_login {
                    login_item::install()
                } else {
                    login_item::uninstall()
                };
                match result {
                    Ok(()) => {
                        self.status = None;
                        self.save_config();
                    }
                    Err(e) => {
                        self.config.start_on_login = previous;
                        self.status = Some(e.to_string());
                    }
                }
            }

            if self.audio.is_device_lost() {
                ui.colored_label(egui::Color32::RED, "Audio device lost — restart keyclack");
            }

            if let Some(status) = &self.status {
                ui.colored_label(egui::Color32::RED, status);
            }
        });
    }
}
