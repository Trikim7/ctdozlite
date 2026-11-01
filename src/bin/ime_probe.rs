//! Offline native IME check: no session, network or message sending.
use eframe::egui;
struct Probe {
    text: String,
    events: Vec<String>,
    output: std::path::PathBuf,
    ime: zalo_rust::input_method::ImeState,
    sends: usize,
}
impl eframe::App for Probe {
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        ctx.input(|i| self.ime.update(&i.events));
        self.events.extend(ctx.input(|i| {
            i.events
                .iter()
                .filter(|e| matches!(e, egui::Event::Ime(_)))
                .map(|e| format!("{e:?}"))
                .collect::<Vec<_>>()
        }));
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.label("Ô thử bộ gõ, không kết nối Zalo và không gửi tin nhắn.");
            let response = ui.add(
                egui::TextEdit::singleline(&mut self.text)
                    .id(egui::Id::new("ime_probe"))
                    .desired_width(500.0),
            );
            if self.ime.may_submit_enter()
                && (response.has_focus() || response.lost_focus())
                && ui.input(|i| i.key_pressed(egui::Key::Enter))
            {
                self.sends += 1;
            }
            response.request_focus();
        });
        let result = serde_json::json!({"text":self.text,"ime_events":self.events,"focused":ctx.input(|i|i.focused),"send_count":self.sends});
        let _ =
            zalo_rust::storage::atomic_write(&self.output, &serde_json::to_vec(&result).unwrap());
        ctx.request_repaint_after(std::time::Duration::from_millis(60));
    }
}
fn main() -> eframe::Result {
    let path = std::env::args()
        .nth(1)
        .expect("Pass a temporary result file path");
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([560.0, 150.0]),
        ..Default::default()
    };
    eframe::run_native(
        "ctdozlite offline IME probe",
        options,
        Box::new(move |_| {
            Ok(Box::new(Probe {
                text: String::new(),
                events: Vec::new(),
                output: path.into(),
                ime: Default::default(),
                sends: 0,
            }))
        }),
    )
}
