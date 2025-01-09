use std::fmt::Formatter;

use eframe::{
    egui::{Align2, CentralPanel, Color32, FontId, TopBottomPanel},
    epaint::PathStroke,
    App, CreationContext,
};

use hulk_manifest::collect_hulk_cyclers;
use repository::Repository;
use source_analyzer::{contexts::Field, cyclers::Cyclers};

pub struct DependencyInspector {
    _repository: Repository,
    cyclers: Cyclers,
    selected_cycler: usize,
    selected_node_index: Option<usize>,
}

struct NamedIndex<'a> {
    index: usize,
    name: &'a str,
}

impl std::fmt::Display for NamedIndex<'_> {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        formatter.write_str(self.name)
    }
}

impl DependencyInspector {
    pub fn new(_creation_context: &CreationContext, repository: Repository) -> Self {
        let cyclers = collect_hulk_cyclers(repository.crates_directory()).unwrap();
        Self {
            _repository: repository,
            cyclers,
            selected_cycler: 0,
            selected_node_index: None,
        }
    }
}

impl App for DependencyInspector {
    fn update(&mut self, context: &eframe::egui::Context, _frame: &mut eframe::Frame) {
        TopBottomPanel::top("top_bar").show(context, |ui| {
            let cycler_names: Vec<_> = self
                .cyclers
                .cyclers
                .iter()
                .enumerate()
                .map(|(index, cycler)| NamedIndex {
                    index,
                    name: &cycler.name,
                })
                .collect();
            self.selected_cycler =
                hulk_widgets::SegmentedControl::new("cycler selector", &cycler_names)
                    .ui(ui)
                    .inner
                    .index
        });
        CentralPanel::default().show(context, |ui| {
            let cycler = self.cyclers.cyclers.get(self.selected_cycler).unwrap();
            ui.label(format!("{} {}", cycler.name, cycler.cycle_nodes.len()));

            let nodes: Vec<_> = cycler
                .setup_nodes
                .iter()
                .chain(&cycler.cycle_nodes)
                .collect();

            ui.add_space(5.0);
            let mut node_points = Vec::new();
            for (index, node) in nodes.iter().enumerate() {
                let label = ui.label(&node.name);
                if label.clicked {
                    self.selected_node_index = Some(index);
                }
                node_points.push(label.rect.right_center());
                ui.add_space(5.0);
            }

            let Some(selected_node_index) = self.selected_node_index else {
                return;
            };
            let selected_node = nodes[selected_node_index];

            let mut cross_inputs = Vec::new();
            let painter = ui.painter();
            for field in &selected_node.contexts.cycle_context {
                let path = match field {
                    Field::Input {
                        cycler_instance,
                        path,
                        ..
                    }
                    | Field::RequiredInput {
                        cycler_instance,
                        path,
                        ..
                    } => {
                        if cycler_instance.is_some() {
                            cross_inputs.push(path);
                            continue;
                        }
                        path
                    }
                    Field::HistoricInput { path, .. } => path,
                    _ => {
                        continue;
                    }
                };
                for (node_index, node) in nodes.iter().enumerate() {
                    if let Some(output) =
                        node.contexts
                            .main_outputs
                            .iter()
                            .find_map(|output| match output {
                                Field::MainOutput { name, .. }
                                    if *name == path.segments.first().unwrap().name =>
                                {
                                    Some(name)
                                }
                                _ => None,
                            })
                    {
                        let a = node_points[selected_node_index];
                        let b = node_points[node_index];
                        painter.line_segment([a, b], PathStroke::new(1.0, Color32::RED));
                        painter.text(
                            (a + b.to_vec2()) / 2.0,
                            Align2::LEFT_CENTER,
                            output,
                            FontId::default(),
                            Color32::LIGHT_GRAY,
                        );
                    }
                }
            }
            for field in &selected_node.contexts.main_outputs {
                let name = match field {
                    Field::MainOutput { name, .. } => name,
                    _ => {
                        continue;
                    }
                };
                for (node_index, node) in nodes.iter().enumerate() {
                    if let Some(output) = node.contexts.cycle_context.iter().find_map(|output| {
                        let path = match output {
                            Field::Input {
                                cycler_instance,
                                path,
                                ..
                            }
                            | Field::RequiredInput {
                                cycler_instance,
                                path,
                                ..
                            } => {
                                if cycler_instance.is_some() {
                                    return None;
                                }
                                path
                            }
                            Field::HistoricInput { path, .. } => path,
                            _ => {
                                return None;
                            }
                        };
                        (*name == path.segments.first().unwrap().name).then_some(name)
                    }) {
                        let a = node_points[selected_node_index];
                        let b = node_points[node_index];
                        painter.line_segment([a, b], PathStroke::new(1.0, Color32::YELLOW));
                        painter.text(
                            (a + b.to_vec2()) / 2.0,
                            Align2::LEFT_CENTER,
                            output,
                            FontId::default(),
                            Color32::LIGHT_GRAY,
                        );
                    }
                }
            }
        });
    }
}
