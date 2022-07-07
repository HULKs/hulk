use std::sync::Arc;

use eframe::Storage;

use crate::nao::Nao;

pub trait Panel {
    const NAME: &'static str;
    fn new(nao: Arc<Nao>, storage: Option<&dyn Storage>) -> Self;
    fn save(&mut self, _storage: &mut dyn Storage) {}
}
