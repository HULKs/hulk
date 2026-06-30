#[macro_export]
macro_rules! impl_selectable_panel {
    ($($name:ident),* $(,)?) => {
        pub enum SelectablePanel {
            $(
                $name(Box<$name>),
            )*
        }

        impl SelectablePanel {
            pub fn new(context: $crate::panel::PanelCreationContext<'_>) -> color_eyre::Result<Self> {
                let saved: $crate::panel::SavedPanel = serde_json::from_value(
                    context
                        .value
                        .cloned()
                        .ok_or_else(|| color_eyre::eyre::eyre!("missing saved panel state"))?,
                )?;
                Self::try_from_id(&saved.kind, $crate::panel::PanelCreationContext {
                    backend: context.backend,
                    value: Some(&saved.state),
                    egui_context: context.egui_context,
                })
            }

            pub fn try_from_id(
                storage_id: &str,
                context: $crate::panel::PanelCreationContext<'_>,
            ) -> color_eyre::Result<Self> {
                match storage_id {
                    $(
                        <$name as $crate::panel::Panel>::STORAGE_ID => Ok(Self::$name(Box::new($name::new(context)))),
                    )*
                    _ => color_eyre::eyre::bail!("unknown panel storage id: {storage_id}"),
                }
            }

            pub fn registered() -> Vec<&'static str> {
                vec![$(<$name as $crate::panel::Panel>::DISPLAY_NAME),*]
            }

            pub fn storage_ids() -> Vec<&'static str> {
                vec![$(<$name as $crate::panel::Panel>::STORAGE_ID),*]
            }

            pub fn try_from_display_name(
                display_name: &str,
                context: $crate::panel::PanelCreationContext<'_>,
            ) -> color_eyre::Result<Self> {
                match display_name {
                    $(
                        <$name as $crate::panel::Panel>::DISPLAY_NAME => Ok(Self::$name(Box::new($name::new(context)))),
                    )*
                    _ => color_eyre::eyre::bail!("unknown panel display name: {display_name}"),
                }
            }

            pub fn save(&self) -> serde_json::Value {
                let saved = match self {
                    $(
                        Self::$name(panel) => $crate::panel::SavedPanel {
                            kind: <$name as $crate::panel::Panel>::STORAGE_ID.to_string(),
                            state: panel.save(),
                        },
                    )*
                };
                serde_json::to_value(saved).expect("saved panel should serialize")
            }
        }

        impl SelectablePanel {
            pub fn ui(
                &mut self,
                ui: &mut eframe::egui::Ui,
                context: $crate::panel::PanelUiContext<'_>,
            ) {
                match self {
                    $(
                        SelectablePanel::$name(panel) => panel.ui(ui, context),
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
