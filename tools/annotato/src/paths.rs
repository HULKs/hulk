use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct Paths {
    pub image_path: PathBuf,
    pub label_path: PathBuf,

    pub image_present: bool,
    pub label_present: bool,
}

impl Paths {
    pub fn new(image_path: PathBuf, label_path: PathBuf) -> Self {
        Self {
            image_present: image_path.exists(),
            label_present: label_path.exists(),
            image_path,
            label_path,
        }
    }

    pub fn check_existence(&mut self) {
        self.image_present = self.image_path.exists();
        self.label_present = self.label_path.exists();
    }
}
