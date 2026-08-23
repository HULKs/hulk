use std::{env::current_dir, path::PathBuf, str::FromStr, sync::Arc};

use clap::Parser;
use color_eyre::{
    Result,
    eyre::{Context as _, ContextCompat as _},
};
use configuration::{
    Configuration,
    keybind_plugin::{self, KeybindSystem},
    keys::KeybindAction,
};
use eframe::{
    App, CreationContext, Frame, NativeOptions, Storage,
    egui::{CentralPanel, Layout, Panel as EguiPanel, Ui},
    emath::Align,
    run_native,
};
use layout::{FocusDirection, TwixLayout};
use log::{error, warn};
use panels::{ImagePanel, MapPanel, TextPanel};
use repository::{Repository, inspect_version::check_for_update};
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};
use visuals::Visuals;

use crate::backend::RobotBackend;

mod backend;
mod configuration;
mod graph;
mod layout;
mod panel;
mod panels;
mod repaint;
mod selectable_panel_macro;
mod status;
mod twix_painter;
mod visuals;
mod zoom_and_pan;

impl_selectable_panel!(TextPanel, ImagePanel, MapPanel);

#[derive(Debug, Clone, clap::Parser)]
struct Arguments {
    /// Target ROS-Z namespace, for example /42.
    namespace: Option<String>,

    /// Router endpoint passed to ROS-Z, for example tcp/127.0.0.1:7447.
    #[arg(long)]
    router: Option<String>,

    /// Alternative repository root for local Twix version checks.
    #[arg(long)]
    repository_root: Option<PathBuf>,

    /// Delete the current panel setup.
    #[arg(long)]
    clear: bool,
}

struct TwixApp {
    layout: TwixLayout,
    namespace_editor: String,
    visual: Visuals,
    backend: Arc<RobotBackend>,
    runtime: tokio::runtime::Runtime,
}

impl TwixApp {
    fn create(
        creation_context: &CreationContext,
        arguments: Arguments,
        runtime: tokio::runtime::Runtime,
        backend: Arc<RobotBackend>,
        configuration: Configuration,
    ) -> Self {
        let namespace_editor = backend.namespace();

        let layout = match arguments.clear {
            true => TwixLayout::new(&creation_context.egui_ctx, &backend),
            false => TwixLayout::load(creation_context, &backend),
        };

        keybind_plugin::register(&creation_context.egui_ctx);
        creation_context
            .egui_ctx
            .set_keybinds(Arc::new(configuration.keys));

        let visual = creation_context
            .storage
            .and_then(|storage| storage.get_string("style"))
            .and_then(|theme| Visuals::from_str(&theme).ok())
            .unwrap_or(Visuals::Dark);
        visual.set_visual(&creation_context.egui_ctx);

        Self {
            layout,
            namespace_editor,
            visual,
            backend,
            runtime,
        }
    }
}

impl App for TwixApp {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut Frame) {
        let _runtime_guard = self.runtime.enter();
        let context = ui.ctx().clone();
        if context.keybind_pressed(KeybindAction::FocusPanel) {
            self.layout.open_selector(&context);
        }
        if context.keybind_pressed(KeybindAction::FocusLeft) {
            self.layout.focus(FocusDirection::Left, &context);
        }
        if context.keybind_pressed(KeybindAction::FocusBelow) {
            self.layout.focus(FocusDirection::Below, &context);
        }
        if context.keybind_pressed(KeybindAction::FocusAbove) {
            self.layout.focus(FocusDirection::Above, &context);
        }
        if context.keybind_pressed(KeybindAction::FocusRight) {
            self.layout.focus(FocusDirection::Right, &context);
        }

        EguiPanel::top("top_bar").show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                    ui.label("Namespace:");
                    let namespace_response = ui.text_edit_singleline(&mut self.namespace_editor);
                    if context.keybind_pressed(KeybindAction::FocusNamespace) {
                        namespace_response.request_focus();
                    }
                    if namespace_response.lost_focus()
                        && self.namespace_editor != self.backend.namespace()
                        && let Err(error) =
                            self.backend.set_namespace(self.namespace_editor.clone())
                    {
                        log::error!("failed to set namespace: {error:#}");
                        self.namespace_editor = self.backend.namespace();
                    }
                });

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.menu_button("Settings", |ui| {
                        ui.menu_button("Theme", |ui| {
                            ui.vertical(|ui| {
                                for visual in Visuals::iter() {
                                    if ui.button(visual.to_string()).clicked() {
                                        self.visual = visual;
                                        self.visual.set_visual(&context);
                                    }
                                }
                            })
                        });
                    });
                });
            })
        });

        CentralPanel::default().show(ui, |ui| {
            if context.keybind_pressed(KeybindAction::OpenSplit) {
                self.layout.open_split(&self.backend, ui.ctx());
            }
            if context.keybind_pressed(KeybindAction::OpenTab) {
                self.layout.open_tab(&self.backend, ui.ctx());
            }

            if context.keybind_pressed(KeybindAction::DuplicateTab) {
                self.layout.duplicate_focused(&self.backend, ui.ctx());
            }

            if context.keybind_pressed(KeybindAction::CloseTab) {
                self.layout.close_focused(&self.backend, ui.ctx());
            }

            if context.keybind_pressed(KeybindAction::CloseAll) {
                self.layout.reset(&self.backend, ui.ctx());
            }

            self.layout.ui(ui, &self.backend);
        });
    }

    fn save(&mut self, storage: &mut dyn Storage) {
        self.layout.save(storage);
        storage.set_string("namespace", self.backend.namespace());
        storage.set_string("style", self.visual.to_string());
    }
}

fn setup_logger() -> Result<()> {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,bevy_render=warn"));
    let layer = fmt::layer()
        .with_target(true)
        .with_thread_ids(false)
        .with_level(true)
        .with_file(false)
        .with_line_number(true)
        .compact();

    tracing_subscriber::registry()
        .with(filter)
        .with(layer)
        .try_init()
        .wrap_err("failed to initialize tracing subscriber")?;

    Ok(())
}

fn main() -> eframe::Result<()> {
    color_eyre::install().expect("failed to install color-eyre");
    setup_logger().expect("failed to setup logger");
    let arguments = Arguments::parse();
    let repository = arguments
        .repository_root
        .clone()
        .map(Repository::new)
        .map(Ok)
        .unwrap_or_else(|| {
            let current_directory = current_dir().wrap_err("failed to get current directory")?;
            Repository::find_root(current_directory).wrap_err("failed to find repository root")
        });
    match &repository {
        Ok(repository) => {
            if let Err(error) = check_for_update(
                env!("CARGO_PKG_VERSION"),
                repository.root.join("tools/twix/Cargo.toml"),
                "twix",
            ) {
                error!("{error:#}");
            }
        }
        Err(error) => {
            warn!("{error:#}");
        }
    }

    let configuration = Configuration::load()
        .unwrap_or_else(|error| panic!("failed to load configuration: {error}"));
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("failed to build Tokio runtime");
    let runtime_handle = runtime.handle().clone();

    run_native(
        "Twix",
        NativeOptions::default(),
        Box::new(move |creation_context| {
            egui_extras::install_image_loaders(&creation_context.egui_ctx);
            egui_material_icons::initialize(&creation_context.egui_ctx);
            let namespace = arguments
                .namespace
                .clone()
                .or_else(|| creation_context.storage?.get_string("namespace"))
                .unwrap_or_else(|| "/".to_string());
            let backend = runtime.block_on(RobotBackend::new(
                runtime_handle.clone(),
                arguments.router.clone(),
                namespace,
            ))?;
            Ok(Box::new(TwixApp::create(
                creation_context,
                arguments.clone(),
                runtime,
                Arc::new(backend),
                configuration,
            )))
        }),
    )
}
