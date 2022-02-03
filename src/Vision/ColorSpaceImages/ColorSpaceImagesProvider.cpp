#include "Tools/Chronometer.hpp"
#include "Tools/Storage/Image.hpp"

#include "Vision/ColorSpaceImages/ColorSpaceImagesProvider.hpp"

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
  sendGrayscaleImage(image, "Y", [](const Color& color) { return color.y; });
  sendGrayscaleImage(image, "Cb", [](const Color& color) { return color.cb; });
  sendGrayscaleImage(image, "Cr", [](const Color& color) { return color.cr; });

  sendGrayscaleImage(image, "R", [](const Color& color) {
    return RGBColor{YCbCr422{color.y, color.y, color.cb, color.cr}}.r;
  });
  sendGrayscaleImage(image, "G", [](const Color& color) {
    return RGBColor{YCbCr422{color.y, color.y, color.cb, color.cr}}.g;
  });
  sendGrayscaleImage(image, "B", [](const Color& color) {
    return RGBColor{YCbCr422{color.y, color.y, color.cb, color.cr}}.b;
  });

  sendGrayscaleImage(image, "rChrom", [](const Color& color) {
    const RGBColor rgb{YCbCr422(color.y, color.y, color.cb, color.cr)};
    return static_cast<std::uint8_t>(rgb.getChromaticity(rgb.r) * 255.f);
  });
  sendGrayscaleImage(image, "gChrom", [](const Color& color) {
    const RGBColor rgb{YCbCr422(color.y, color.y, color.cb, color.cr)};
    return static_cast<std::uint8_t>(rgb.getChromaticity(rgb.g) * 255.f);
  });
  sendGrayscaleImage(image, "bChrom", [](const Color& color) {
    const RGBColor rgb{YCbCr422(color.y, color.y, color.cb, color.cr)};
    return static_cast<std::uint8_t>(rgb.getChromaticity(rgb.b) * 255.f);
  });
}

void ColorSpaceImagesProvider::sendGrayscaleImage(
    const Image422& image, const std::string& name,
    const std::function<std::uint8_t(const Color&)>& getValue) const
{
  // This only sends every third image because the drawing takes a lot of processing time
  if (debug().isSubscribed(mount_ + "." + name + "." + imageData_->identification))
  {
    Image debugImage(Image422::get444From422Vector(image.size));
    for (int y = 0; y < debugImage.size.y(); y++)
    {
      for (int x = 0; x < debugImage.size.x(); x++)
      {
        const YCbCr422& yCbCr422Pixel = image.at(y, x / 2);
        const Color pixelColor{(x % 2) != 0 ? yCbCr422Pixel.y2 : yCbCr422Pixel.y1, yCbCr422Pixel.cb,
                               yCbCr422Pixel.cr};
        debugImage.at(y, x) = Color{getValue(pixelColor), 128, 128};
      }
    }
    debug().sendImage(mount_ + "." + name + "." + imageData_->identification, debugImage);
  }
}
