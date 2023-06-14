use std::str::FromStr;

use eframe::egui::{self, Context};

#[derive(Debug)]
pub enum Visuals {
    Dark,
    Light,
}

impl Visuals {
    pub fn iter() -> Vec<Visuals> {
        vec![Visuals::Dark, Visuals::Light]
    }

    pub fn set_visual(&self, context: &Context) {
        context.set_visuals(self.theme());
    }

    pub fn theme(&self) -> egui::Visuals {
        match self {
            Visuals::Dark => egui::Visuals::dark(),
            Visuals::Light => egui::Visuals::light(),
        }
    }
}

impl ToString for Visuals {
    fn to_string(&self) -> String {
        match self {
            Visuals::Dark => "🌑 Dark",
            Visuals::Light => "☀ Light",
        }
        .to_owned()
    }
}

impl FromStr for Visuals {
    type Err = String;

    fn from_str(string: &str) -> Result<Self, Self::Err> {
        match string {
            "🌑 Dark" => Ok(Visuals::Dark),
            "☀ Light" => Ok(Visuals::Light),
            theme @ _ => Err(format!("{theme} is unknown")),
        }
    }
}
