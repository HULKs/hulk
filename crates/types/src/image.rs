use color_eyre::Result;
use std::{
    ops::{Index, IndexMut},
    path::Path,
};

use image::{io::Reader, Rgb, RgbImage};
use nalgebra::{point, Point2};

use super::color::YCbCr422;

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct Image422 {
    pixels: Vec<YCbCr422>,
    width: usize,
    height: usize,
}

impl Image422 {
    pub fn zero(width: usize, height: usize) -> Self {
        Self {
            pixels: vec![Default::default(); width * height],
            width,
            height,
        }
    }

    #[allow(dead_code)]
    pub fn from_slice(data: &[u8], width: usize, height: usize) -> Self {
        Self {
            pixels: data
                .chunks(4)
                .map(|x| YCbCr422 {
                    y1: x[0],
                    cb: x[1],
                    y2: x[2],
                    cr: x[3],
                })
                .collect(),
            width,
            height,
        }
    }

    #[allow(dead_code)]
    pub fn pixels_as_mut_slice(&mut self) -> &mut [YCbCr422] {
        self.pixels.as_mut_slice()
    }

    pub fn load_from_ycbcr_444_file<P>(file: P) -> Result<Image422>
    where
        P: AsRef<Path>,
    {
        let png = Reader::open(file)?.decode()?.into_rgb8();

        let width = png.width();
        let height = png.height();
        let rgb_pixels = png.into_vec();

        let pixels = rgb_pixels
            .chunks(6)
            .map(|x| YCbCr422 {
                y1: x[0],
                cb: ((x[1] as u16 + x[4] as u16) / 2) as u8,
                y2: x[3],
                cr: ((x[2] as u16 + x[5] as u16) / 2) as u8,
            })
            .collect();

        Ok(Image422 {
            pixels,
            width: width as usize / 2,
            height: height as usize,
        })
    }

    pub fn save_to_ycbcr_444_file<P>(&self, file: P) -> Result<()>
    where
        P: AsRef<Path>,
    {
        let mut image = RgbImage::new(self.width() as u32 * 2, self.height() as u32);
        for y in 0..self.height() {
            for x in 0..self.width() {
                let pixel = self[(x, y)];
                image.put_pixel(x as u32 * 2, y as u32, Rgb([pixel.y1, pixel.cb, pixel.cr]));
                image.put_pixel(
                    x as u32 * 2 + 1,
                    y as u32,
                    Rgb([pixel.y2, pixel.cb, pixel.cr]),
                );
            }
        }
        Ok(image.save(file)?)
    }

    #[allow(dead_code)]
    pub fn width(&self) -> usize {
        self.width
    }

    #[allow(dead_code)]
    pub fn height(&self) -> usize {
        self.height
    }

    pub fn is_inside(&self, position: Point2<usize>) -> bool {
        let x_inside = position.x < self.width;
        let y_inside = position.y < self.height;
        x_inside && y_inside
    }

    pub fn try_at(&self, position: Point2<f32>) -> Option<YCbCr422> {
        let position = point![position.x as usize, position.y as usize];
        if !self.is_inside(position) {
            return None;
        }
        Some(self[position])
    }
}

impl Index<(usize, usize)> for Image422 {
    type Output = YCbCr422;

    fn index(&self, (x, y): (usize, usize)) -> &Self::Output {
        &self.pixels[y * self.width + x]
    }
}

impl IndexMut<(usize, usize)> for Image422 {
    fn index_mut(&mut self, (x, y): (usize, usize)) -> &mut Self::Output {
        &mut self.pixels[y * self.width + x]
    }
}

impl Index<Point2<usize>> for Image422 {
    type Output = YCbCr422;

    fn index(&self, position: Point2<usize>) -> &Self::Output {
        &self.pixels[position.y * self.width + position.x]
    }
}

impl IndexMut<Point2<usize>> for Image422 {
    fn index_mut(&mut self, position: Point2<usize>) -> &mut Self::Output {
        &mut self.pixels[position.y * self.width + position.x]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn image_has_zero_constructor() {
        let image = Image422::zero(10, 12);
        assert!(image.pixels.into_iter().all(|x| x == YCbCr422::default()));
    }

    #[test]
    fn image_has_width_and_height() {
        let image = Image422::zero(10, 12);
        assert_eq!(image.width(), 10);
        assert_eq!(image.height(), 12);
    }

    #[test]
    fn image_can_be_indexed() {
        let mut image = Image422::zero(10, 12);
        image[(1, 1)] = YCbCr422 {
            y1: 1,
            cb: 2,
            y2: 3,
            cr: 4,
        };
        assert_eq!(
            image[(1, 1)],
            YCbCr422 {
                y1: 1,
                cb: 2,
                y2: 3,
                cr: 4
            }
        );
    }
}
