use std::path::{Path, PathBuf};

use color_eyre::Result;
use eframe::{
    egui::Ui,
    epaint::{ColorImage, TextureHandle},
};
use image::{io::Reader, ImageError};

pub fn load_image_from_path(path: impl AsRef<Path>) -> Result<ColorImage, ImageError> {
    let image = Reader::open(path)?.decode()?;
    let size = [image.width() as _, image.height() as _];
    let image_buffer = image.to_rgba8();
    let pixels = image_buffer.as_flat_samples();
    Ok(ColorImage::from_rgba_unmultiplied(size, pixels.as_slice()))
}

pub fn load_image(ui: &Ui, image_path: &PathBuf) -> Result<TextureHandle> {
    let image = load_image_from_path(image_path)?;
    let handle = ui
        .ctx()
        .load_texture(image_path.display().to_string(), image, Default::default());

    Ok(handle)
}
