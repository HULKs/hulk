use std::{
    fmt::{self, Display, Formatter},
    str::FromStr,
};

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

impl Display for Visuals {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Visuals::Dark => "🌑 Dark",
            Visuals::Light => "☀ Light",
        })
    }
}

impl FromStr for Visuals {
    type Err = String;

    fn from_str(string: &str) -> Result<Self, Self::Err> {
        match string {
            "🌑 Dark" => Ok(Visuals::Dark),
            "☀ Light" => Ok(Visuals::Light),
            theme => Err(format!("{theme} is unknown")),
        }
    }
}
