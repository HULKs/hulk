#[macro_export]
macro_rules! impl_selectable_panel {
    ($($name:ident),* $(,)?) => {
        #[allow(clippy::large_enum_variant)]
        pub enum SelectablePanel {
            $(
                $name($name),
            )*
        }

        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub enum PanelKind {
            $(
                $name,
            )*
        }

        impl PanelKind {
            pub fn from_storage_id(storage_id: &str) -> color_eyre::Result<Self> {
                match storage_id {
                    $(
                        <$name as $crate::panel::Panel>::STORAGE_ID => Ok(Self::$name),
                    )*
                    _ => color_eyre::eyre::bail!("unknown panel storage id: {storage_id}"),
                }
            }

            pub const fn storage_id(self) -> &'static str {
                match self {
                    $(
                        Self::$name => <$name as $crate::panel::Panel>::STORAGE_ID,
                    )*
                }
            }

            pub const fn display_name(self) -> &'static str {
                match self {
                    $(
                        Self::$name => <$name as $crate::panel::Panel>::DISPLAY_NAME,
                    )*
                }
            }

            pub const fn icon(self) -> &'static str {
                match self {
                    $(
                        Self::$name => <$name as $crate::panel::Panel>::ICON,
                    )*
                }
            }

            pub fn label(self) -> &'static str {
                static LABELS: std::sync::LazyLock<Vec<(PanelKind, String)>> =
                    std::sync::LazyLock::new(|| {
                        PanelKind::registered()
                            .iter()
                            .map(|kind| (*kind, format!("{}  {}", kind.icon(), kind.display_name())))
                            .collect()
                    });
                LABELS
                    .iter()
                    .find_map(|(kind, label)| (*kind == self).then_some(label.as_str()))
                    .unwrap_or_else(|| self.display_name())
            }

            pub fn create(
                self,
                context: $crate::panel::PanelCreationContext<'_>,
            ) -> SelectablePanel {
                match self {
                    $(
                        Self::$name => SelectablePanel::$name(
                            <$name as $crate::panel::Panel>::new(context)
                        ),
                    )*
                }
            }

            pub fn registered() -> &'static [Self] {
                const PANELS: &[PanelKind] = &[
                    $(
                        PanelKind::$name,
                    )*
                ];
                PANELS
            }
        }

        impl std::fmt::Display for PanelKind {
            fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str(self.display_name())
            }
        }

        impl SelectablePanel {
            pub fn new(context: $crate::panel::PanelCreationContext<'_>) -> color_eyre::Result<Self> {
                let saved: $crate::panel::SavedPanel = serde_json::from_value(
                    context
                        .value
                        .cloned()
                        .ok_or_else(|| color_eyre::eyre::eyre!("missing saved panel state"))?,
                )?;
                let kind = PanelKind::from_storage_id(&saved.kind)?;
                Ok(kind.create($crate::panel::PanelCreationContext {
                    backend: context.backend,
                    value: Some(&saved.state),
                    egui_context: context.egui_context,
                }))
            }

            pub fn kind(&self) -> PanelKind {
                match self {
                    $(
                        Self::$name(_) => PanelKind::$name,
                    )*
                }
            }

            pub fn save(&self) -> serde_json::Value {
                let saved = match self {
                    $(
                        Self::$name(panel) => $crate::panel::SavedPanel {
                            kind: <$name as $crate::panel::Panel>::STORAGE_ID.to_string(),
                            state: <$name as $crate::panel::Panel>::save(panel),
                        },
                    )*
                };
                serde_json::to_value(saved).expect("saved panel should serialize")
            }
        }

        impl SelectablePanel {
            pub fn header_ui(
                &mut self,
                ui: &mut eframe::egui::Ui,
                context: $crate::panel::PanelUiContext<'_>,
            ) {
                match self {
                    $(
                        SelectablePanel::$name(panel) => {
                            <$name as $crate::panel::Panel>::header_ui(panel, ui, context)
                        },
                    )*
                }
            }

            pub fn ui(
                &mut self,
                ui: &mut eframe::egui::Ui,
                context: $crate::panel::PanelUiContext<'_>,
            ) {
                match self {
                    $(
                        SelectablePanel::$name(panel) => {
                            <$name as $crate::panel::Panel>::ui(panel, ui, context)
                        },
                    )*
                }
            }
        }

        impl std::fmt::Display for SelectablePanel {
            fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                let panel_name = match self {
                    $(
                        Self::$name(_) => <$name as $crate::panel::Panel>::DISPLAY_NAME,
                    )*
                };
                formatter.write_str(panel_name)
            }
        }
    };
}
