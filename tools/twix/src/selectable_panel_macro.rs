#[macro_export]
macro_rules! impl_selectable_panel {
    ($($name:ident),* $(,)?) => {
        #[allow(clippy::large_enum_variant)]
        pub enum SelectablePanel {
            $(
                $name ($name)
            ),*
        }

        impl SelectablePanel {
            fn new(nao: Arc<Nao>, value: Option<&Value>) -> Result<SelectablePanel> {
                let name = value
                    .ok_or(eyre!("Got none value"))?
                    .get("_panel_type")
                    .ok_or(eyre!("value has no _panel_type: {value:?}"))?
                    .as_str()
                    .ok_or(eyre!("_panel_type is not a string"))?;
                Self::try_from_name(&name.to_owned(), nao, value)
            }

            pub fn try_from_name(panel_name: &String, nao: Arc<Nao>, value: Option<&Value>) -> Result<SelectablePanel> {
                match panel_name.as_str() {
                    $(
                        $name::NAME => Ok(SelectablePanel::$name($name::new(nao, value))),
                    )*
                    _ => bail!("{panel_name} panel unknown"),
                }
            }

            pub fn registered() -> Vec<String> {
                vec![
                    $(
                        $name::NAME.to_owned()
                    ),*
                ]
            }

            pub fn save(&self) -> Value {
                let mut value = match self {
                    $(
                        SelectablePanel::$name(panel) => panel.save(),
                    )*
                };

                value["_panel_type"] = Value::String(self.to_string());

                value
            }
        }

        impl Widget for &mut SelectablePanel {
            fn ui(self, ui: &mut Ui) -> eframe::egui::Response {
                match self {
                    $(
                        SelectablePanel::$name(panel) => panel.ui(ui),
                    )*
                }
            }
        }

        impl std::fmt::Display for SelectablePanel {
            fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                let panel_name = match self {
                    $(
                        SelectablePanel::$name(_) => $name::NAME,
                    )*
                };
                formatter.write_str(panel_name)
            }
        }
    };
}
