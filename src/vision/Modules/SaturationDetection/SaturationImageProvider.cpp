#include "Tools/Chronometer.hpp"
#include "Tools/Storage/Image.hpp"

#include "SaturationImageProvider.hpp"

#include <limits>

SaturationImageProvider::SaturationImageProvider(const ModuleManagerInterface& manager)
  : Module(manager)
  , imageData_(*this)
  , counter_(0)

{
}

void SaturationImageProvider::cycle()
{
  Chronometer time(debug(), mount_ + ".cycleTime");
  const Image422& image = imageData_->image422;

  sendImageForDebug(image);
}

void SaturationImageProvider::sendImageForDebug(const Image422& image)
{
  if (!debug().isSubscribed(mount_ + "." + imageData_->identification + "_image"))
  {
    return;
  }

  if (!(counter_++ % 3))
  {
    // This only sends every third image because the
    // drawing takes a lot of processing time
    Image saturationImage(image.to444Image());
    for (int y = 0; y < saturationImage.size_.y(); y++)
    {
      for (int x = 0; x < saturationImage.size_.x(); x++)
      {
        const RGBColor RGBvalues = image.at(y, x / 2).RGB();
        if (RGBvalues.isSaturated())
        {
          saturationImage.at(Vector2i(x, y)) = Color::PINK;
        }
      }
    }

    debug().sendImage(mount_ + "." + imageData_->identification + "_image", saturationImage);
  }
}
