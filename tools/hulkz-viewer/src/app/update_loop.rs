use eframe::{App, Frame, Storage};

use super::{persistence::save_persisted_state, state::ViewerApp};

impl App for ViewerApp {
    fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut Frame) {
        let frame_started = std::time::Instant::now();
        self.run_pending_commands();
        self.drain_worker_events();

        let mut ui_intents = Vec::new();
        self.render_shell_panes(ctx, &mut ui_intents);
        self.render_workspace(ctx, &mut ui_intents);
        self.apply_ui_intents(ui_intents);

        self.reconcile_text_panels();
        self.maybe_emit_scrub_command();

        let frame_ms = frame_started.elapsed().as_secs_f32() * 1000.0;
        self.ui.frame_last_ms = frame_ms;
        self.ui.frame_ema_ms = if self.ui.frame_ema_ms <= f32::EPSILON {
            frame_ms
        } else {
            self.ui.frame_ema_ms * 0.9 + frame_ms * 0.1
        };

        let pending_events = !self.runtime.event_rx.is_empty();
        let pending_commands = !self.runtime.pending_commands.is_empty();
        if pending_events || pending_commands || (self.ui.follow_live && self.ui.ingest_enabled) {
            ctx.request_repaint_after(self.config.repaint_delay_on_activity);
        }
    }

    fn save(&mut self, storage: &mut dyn Storage) {
        save_persisted_state(self, storage);
    }
}
