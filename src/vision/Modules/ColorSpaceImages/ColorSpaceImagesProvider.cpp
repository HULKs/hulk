#include "Tools/Chronometer.hpp"
#include "Tools/Storage/Image.hpp"

#include "ColorSpaceImagesProvider.hpp"

ColorSpaceImagesProvider::ColorSpaceImagesProvider(const ModuleManagerInterface& manager)
  : Module(manager)
  , imageData_(*this)
{
}

void ColorSpaceImagesProvider::cycle()
{
  sendImagesForDebug(imageData_->image422);
}

void ColorSpaceImagesProvider::sendImagesForDebug(const Image422& image) const
{
  sendGrayscaleImage(image, "Y", [](const Color& color) { return color.y_; });
  sendGrayscaleImage(image, "Cb", [](const Color& color) { return color.cb_; });
  sendGrayscaleImage(image, "Cr", [](const Color& color) { return color.cr_; });

  sendGrayscaleImage(image, "R", [](const Color& color) {
    return YCbCr422(color.y_, color.y_, color.cb_, color.cr_).RGB().r;
  });
  sendGrayscaleImage(image, "G", [](const Color& color) {
    return YCbCr422(color.y_, color.y_, color.cb_, color.cr_).RGB().g;
  });
  sendGrayscaleImage(image, "B", [](const Color& color) {
    return YCbCr422(color.y_, color.y_, color.cb_, color.cr_).RGB().b;
  });

  sendGrayscaleImage(image, "rChrom", [](const Color& color) {
    const RGBColor rgb(YCbCr422(color.y_, color.y_, color.cb_, color.cr_).RGB());
    return static_cast<std::uint8_t>(rgb.getChromaticity(rgb.r) * 255.f);
  });
  sendGrayscaleImage(image, "gChrom", [](const Color& color) {
    const RGBColor rgb(YCbCr422(color.y_, color.y_, color.cb_, color.cr_).RGB());
    return static_cast<std::uint8_t>(rgb.getChromaticity(rgb.g) * 255.f);
  });
  sendGrayscaleImage(image, "bChrom", [](const Color& color) {
    const RGBColor rgb(YCbCr422(color.y_, color.y_, color.cb_, color.cr_).RGB());
    return static_cast<std::uint8_t>(rgb.getChromaticity(rgb.b) * 255.f);
  });
}

void ColorSpaceImagesProvider::sendGrayscaleImage(
    const Image422& image, std::string name,
    std::function<std::uint8_t(const Color&)> getValue) const
{
  // This only sends every third image because the drawing takes a lot of processing time
  if (debug().isSubscribed(mount_ + "." + name + "." + imageData_->identification))
  {
    Image debugImage(Image422::get444From422Vector(image.size));
    for (int y = 0; y < debugImage.size_.y(); y++)
    {
      for (int x = 0; x < debugImage.size_.x(); x++)
      {
        const YCbCr422& yCbCr422Pixel = image.at(y, x / 2);
        const Color pixelColor(x % 2 ? yCbCr422Pixel.y2_ : yCbCr422Pixel.y1_, yCbCr422Pixel.cb_,
                               yCbCr422Pixel.cr_);
        debugImage.at(y, x) = Color(getValue(pixelColor), 128, 128);
      }
    }
    debug().sendImage(mount_ + "." + name + "." + imageData_->identification, debugImage);
  }
}
