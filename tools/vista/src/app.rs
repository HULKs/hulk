use std::fmt::Formatter;

use eframe::{
    egui::{pos2, vec2, Align2, CentralPanel, Color32, FontId, ScrollArea, Shape, TopBottomPanel},
    epaint::{PathStroke, QuadraticBezierShape},
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
            let response =
                hulk_widgets::SegmentedControl::new("cycler selector", &cycler_names).ui(ui);
            if response.response.changed() {
                self.selected_cycler = response.inner.index;
                self.selected_node_index = None;
            }
        });

        CentralPanel::default().show(context, |ui| {
            ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    let cycler = self.cyclers.cyclers.get(self.selected_cycler).unwrap();

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
                        node_points.push(label.rect.right_center() + vec2(3.0, 0.0));
                        ui.add_space(5.0);
                    }

                    let minimum_x = node_points
                        .iter()
                        .map(|point| point.x)
                        .max_by(f32::total_cmp)
                        .unwrap_or(0.0)
                        + 5.0;

                    let Some(selected_node_index) = self.selected_node_index else {
                        return;
                    };
                    let selected_node = nodes[selected_node_index];

                    let mut cross_inputs = Vec::new();
                    let painter = ui.painter();
                    let mut count = 0;
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
                                if let Some(cycler_instance) = cycler_instance {
                                    cross_inputs.push((cycler_instance.clone(), path));
                                    continue;
                                }
                                path
                            }
                            Field::PerceptionInput {
                                cycler_instance,
                                path,
                                ..
                            } => {
                                cross_inputs.push((cycler_instance.clone(), path));
                                continue;
                            }
                            Field::HistoricInput { path, .. } => path,
                            _ => {
                                continue;
                            }
                        };
                        for (node_index, node) in nodes.iter().enumerate().rev() {
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
                                let a = pos2(
                                    minimum_x + count as f32 * 10.0,
                                    node_points[selected_node_index].y,
                                );
                                let b = node_points[node_index];
                                let curve = QuadraticBezierShape::from_points_stroke(
                                    [a, pos2(a.x, b.y), b],
                                    false,
                                    Color32::TRANSPARENT,
                                    PathStroke::new(1.0, Color32::RED),
                                );
                                painter.text(
                                    curve.sample(0.9) + vec2(5.0, 0.0),
                                    Align2::LEFT_CENTER,
                                    output,
                                    FontId::default(),
                                    Color32::LIGHT_GRAY,
                                );
                                painter.add(Shape::QuadraticBezier(curve));
                                count += 1;
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
                            if let Some(output) =
                                node.contexts.cycle_context.iter().find_map(|output| {
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
                                })
                            {
                                let a = pos2(
                                    minimum_x + count as f32 * 10.0,
                                    node_points[selected_node_index].y,
                                );
                                let b = node_points[node_index];
                                let curve = QuadraticBezierShape::from_points_stroke(
                                    [a, pos2(a.x, b.y), b],
                                    false,
                                    Color32::TRANSPARENT,
                                    PathStroke::new(1.0, Color32::YELLOW),
                                );
                                painter.text(
                                    curve.sample(0.9) + vec2(5.0, 0.0),
                                    Align2::LEFT_CENTER,
                                    output,
                                    FontId::default(),
                                    Color32::LIGHT_GRAY,
                                );
                                painter.add(Shape::QuadraticBezier(curve));
                                count += 1;
                            }
                        }
                    }

                    for (input_index, (cycler_instance, input)) in cross_inputs.iter().enumerate() {
                        let a = node_points[selected_node_index];
                        let b = painter
                            .text(
                                pos2(
                                    ui.clip_rect().right(),
                                    a.y + -(cross_inputs.len() as f32 / 2.0
                                        - input_index as f32
                                        - 0.5)
                                        * FontId::default().size
                                        * 1.5,
                                ),
                                Align2::RIGHT_CENTER,
                                format!("{} ({})", input.to_segments().join("."), cycler_instance),
                                FontId::default(),
                                Color32::LIGHT_GRAY,
                            )
                            .left_center();
                        painter.line_segment([a, b], PathStroke::new(1.0, Color32::RED));
                    }
                    painter.line_segment(
                        [
                            node_points[selected_node_index],
                            pos2(
                                minimum_x + count as f32 * 10.0 - 10.0,
                                node_points[selected_node_index].y,
                            ),
                        ],
                        PathStroke::new(1.0, Color32::GREEN),
                    );
                });
        });
    }
}
