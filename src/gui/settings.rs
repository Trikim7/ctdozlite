use super::*;

pub(super) struct SettingsGeometry {
    pub rect: Rect,
    pub compact: bool,
}
pub(super) fn settings_geometry(screen: Rect) -> SettingsGeometry {
    let width = (screen.width() - 24.0).min(860.0).max(1.0);
    let height = (screen.height() - 64.0).min(680.0).max(1.0);
    let center = Pos2::new(
        screen.center().x,
        screen.top() + 32.0 + (screen.height() - 32.0) / 2.0,
    );
    let rect = Rect::from_center_size(center, Vec2::new(width, height));
    SettingsGeometry {
        rect,
        compact: width < 680.0,
    }
}
impl ZaloGuiApp {
    pub(super) fn render_call_support(&mut self, ui: &mut egui::Ui, pal: &ThemePalette) {
        if self.call_support.is_none() && !self.checking_call_support {
            self.checking_call_support = true;
            if let Some(tx) = WORKER_TX.get() {
                let _ = tx.send(WorkerCmd::CheckCallSupport);
            }
        }
        ui.label(
            RichText::new("Sẵn sàng cho cuộc gọi")
                .strong()
                .color(pal.text_primary),
        );
        if let Some(report) = self.call_support.as_ref() {
            for (name, ready) in report.checks() {
                ui.horizontal_wrapped(|ui| {
                    let (indicator, _) = ui.allocate_exact_size(Vec2::splat(12.0), Sense::hover());
                    ui.painter().circle_filled(
                        indicator.center(),
                        3.5,
                        if ready {
                            pal.accent_online
                        } else {
                            pal.warning
                        },
                    );
                    ui.label(format!(
                        "{name}: {}",
                        if ready {
                            "Sẵn sàng"
                        } else {
                            "Chưa sẵn sàng"
                        }
                    ));
                });
            }
            if !report.ready_voice() {
                ui.label("Chọn micro và loa khả dụng trong phần thiết bị bên dưới.");
            }
            if !report.ready_video() {
                ui.label("Gọi hình cần camera và FFmpeg hỗ trợ H.264.");
            }
        } else {
            ui.label("Đang kiểm tra thiết bị và thành phần cuộc gọi…");
        }
        if ui
            .add_enabled(
                !self.checking_call_support,
                egui::Button::new("Kiểm tra lại"),
            )
            .clicked()
        {
            self.checking_call_support = true;
            if let Some(tx) = WORKER_TX.get() {
                let _ = tx.send(WorkerCmd::CheckCallSupport);
            }
        }
        ui.add_space(14.0);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn settings_fit_small_and_large_windows() {
        for size in [
            Vec2::new(480.0, 360.0),
            Vec2::new(1020.0, 700.0),
            Vec2::new(1920.0, 1080.0),
        ] {
            let screen = Rect::from_min_size(Pos2::ZERO, size);
            let geometry = settings_geometry(screen);
            assert!(screen.contains_rect(geometry.rect));
            assert_eq!(geometry.compact, size.x < 704.0);
        }
    }
}
