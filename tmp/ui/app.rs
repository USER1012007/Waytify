use egui::Context;

pub struct MusicApp {
    pub playing: bool,
}

impl Default for MusicApp {
    fn default() -> Self {
        Self { playing: false }
    }
}

impl MusicApp {
    pub fn ui(&mut self, ctx: &Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Rust Music Player");

            ui.add_space(20.0);

            if ui.button("Play").clicked() {
                self.playing = true;
                println!("Playing...");
            }

            ui.add_space(10.0);

            if ui.button("Pause").clicked() {
                self.playing = false;
                println!("Paused");
            }

            ui.add_space(20.0);

            ui.label(format!(
                "Status: {}",
                if self.playing { "Playing" } else { "Paused" }
            ));
        });
    }
}
