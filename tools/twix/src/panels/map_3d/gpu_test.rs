use std::{collections::HashSet, sync::Arc};

use eframe::{
    egui::{self, epaint::Primitive},
    egui_wgpu::{RenderState, WgpuConfiguration, WgpuSetup},
    wgpu,
};

use super::Map3DPanel;
use crate::{
    backend::RobotBackend,
    panel::{Panel, PanelCreationContext, PanelUiContext},
};

// Run: cargo test -p twix gpu_multiple_tabs -- --ignored --nocapture
#[test]
#[ignore = "requires a working WGPU adapter"]
fn gpu_multiple_tabs() {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(async {
        let mut config = WgpuConfiguration::default();
        let WgpuSetup::CreateNew(setup) = &mut config.wgpu_setup else {
            panic!("expected default WGPU device creation");
        };
        // Match Twix's Bevy device limit.
        let previous = setup.device_descriptor.clone();
        setup.device_descriptor = Arc::new(move |adapter| {
            let mut descriptor = previous(adapter);
            descriptor
                .required_limits
                .max_storage_buffers_per_shader_stage = 9;
            descriptor
        });
        let instance = config.wgpu_setup.new_instance().await;
        let render_state = RenderState::create(&config, &instance, None, Default::default())
            .await
            .expect("headless WGPU adapter/device creation failed");
        eprintln!("GPU smoke adapter: {:?}", render_state.adapter.get_info());
        let backend = Arc::new(
            RobotBackend::new(runtime.handle().clone(), None, "/".to_string())
                .await
                .unwrap(),
        );
        let context = egui::Context::default();
        let new_panel = || {
            let panel = Map3DPanel::new(PanelCreationContext {
                backend: backend.clone(),
                value: None,
                egui_context: context.clone(),
                render_state: Some(render_state.clone()),
            });
            assert!(panel.widget.is_some());
            assert!(panel.observations.is_ok());
            panel
        };
        let mut panels = [Some(new_panel()), Some(new_panel())];
        for (frame, size) in [
            [800.0, 600.0],
            [1280.0, 900.0],
            [640.0, 480.0],
            [1280.0, 900.0],
            [800.0, 600.0],
        ]
        .into_iter()
        .enumerate()
        {
            if frame == 2 {
                drop(panels[0].take());
            } else if frame == 3 {
                panels[0] = Some(new_panel());
            }
            let output = context.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(size[0], size[1]),
                    )),
                    ..Default::default()
                },
                |ui| {
                    ui.columns(2, |columns| {
                        for (panel, ui) in panels.iter_mut().zip(columns) {
                            if let Some(panel) = panel {
                                panel.ui(
                                    ui,
                                    PanelUiContext {
                                        backend: &backend,
                                        egui_context: &context,
                                    },
                                );
                            }
                        }
                    });
                },
            );
            let textures: HashSet<_> = context
                .tessellate(output.shapes, output.pixels_per_point)
                .into_iter()
                .filter_map(|primitive| match primitive.primitive {
                    Primitive::Mesh(mesh)
                        if matches!(mesh.texture_id, egui::TextureId::User(_)) =>
                    {
                        Some(mesh.texture_id)
                    }
                    _ => None,
                })
                .collect();
            assert_eq!(textures.len(), panels.iter().flatten().count());
            for texture in textures {
                assert!(render_state.renderer.read().texture(&texture).is_some());
            }
            render_state
                .device
                .poll(wgpu::PollType::wait_indefinitely())
                .unwrap();
        }
        drop(panels);
        render_state
            .device
            .poll(wgpu::PollType::wait_indefinitely())
            .unwrap();
    });
}
