use bevy::{
    app::App,
    ecs::system::{ResMut, Resource, SystemParam},
};

#[derive(Clone)]
pub struct SoftError;

#[derive(Default, Resource)]
pub struct SoftErrorResource {
    pub errors: Vec<SoftError>,
}

#[derive(SystemParam)]
pub struct SoftErrorSender<'w> {
    resource: ResMut<'w, SoftErrorResource>,
}

impl SoftErrorSender<'_> {
    pub fn send(&mut self, message: impl Into<String>) {
        let message = message.into();
        println!("{message}");
        if self.resource.errors.is_empty() {
            self.resource.errors.push(SoftError);
        }
    }
}

pub fn soft_error_plugin(app: &mut App) {
    app.insert_resource(SoftErrorResource::default());
}
