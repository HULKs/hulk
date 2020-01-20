#include "Tools/Chronometer.hpp"
#include "Tools/Storage/Image.hpp"

#include "ChromaticityFieldColorDetection.hpp"

ChromaticityFieldColorDetection::ChromaticityFieldColorDetection(
    const ModuleManagerInterface& manager)
  : Module(manager)
  , imageData_(*this)
  , cameraMatrix_(*this)
  , redChromaticityThreshold_(*this, "redChromaticityThreshold",
                              [this] { this->setIsFieldColorFunction(); })
  , lowerGreenChromaticityThreshold_(*this, "lowerGreenChromaticityThreshold",
                                     [this] { this->setIsFieldColorFunction(); })
  , upperGreenChromaticityThreshold_(*this, "upperGreenChromaticityThreshold",
                                     [this] { this->setIsFieldColorFunction(); })
  , blueChromaticityThreshold_(*this, "blueChromaticityThreshold",
                               [this] { this->setIsFieldColorFunction(); })
  , fieldColor_(*this)
{
  setIsFieldColorFunction();
}

void ChromaticityFieldColorDetection::setIsFieldColorFunction()
{
  fieldColor_->isFieldColor = [this](const YCbCr422& pixel) -> float {
    const RGBColor RGBValues = pixel.RGB();
    const float redChromaticity = RGBValues.getChromaticity(RGBValues.r);
    const float greenChromaticity = RGBValues.getChromaticity(RGBValues.g);
    const float blueChromaticity = RGBValues.getChromaticity(RGBValues.b);
    if (redChromaticity < redChromaticityThreshold_() &&
        blueChromaticity < blueChromaticityThreshold_())
    {
      if (greenChromaticity > upperGreenChromaticityThreshold_())
      {
        return 1.f;
      }
      else if (greenChromaticity > lowerGreenChromaticityThreshold_())
      {
        return 0.5f;
      }
    }
    return 0.f;
  };
}

void ChromaticityFieldColorDetection::cycle()
{
  {
    Chronometer time(debug(), mount_ + ".cycleTime");

    if (cameraMatrix_->getHorizonHeight() < imageData_->image422.size.y())
    {
      // The ground is visible at the moment.
      fieldColor_->valid = true;
    }
  }

  sendImageForDebug(imageData_->image422);
}

void ChromaticityFieldColorDetection::sendImageForDebug(const Image422& image)
{
  int horizonY = cameraMatrix_->getHorizonHeight();
  if (!(debugImageCounter_++ % 3))
  {
    // This only sends every third image because the drawing takes a lot of processing time
    if (debug().isSubscribed(mount_ + "." + imageData_->identification + "_image"))
    {
      Image fieldColorImage(image.to444Image());
      for (int y = horizonY; y < fieldColorImage.size_.y(); y += 2)
      {
        for (int x = 0; x < fieldColorImage.size_.x(); ++x)
        {
          const auto fieldColorCertainty = fieldColor_->isFieldColor(image.at(y, x / 2));
          if (fieldColorCertainty == 1.f)
          {
            fieldColorImage.at(Vector2i(x, y)) = Color::YELLOW;
          }
          else if (fieldColorCertainty >= 0.5f)
          {
            fieldColorImage.at(Vector2i(x, y)) = Color::BLUE;
          }
        }
      }

      // draw horizon line
      Vector2i p1(0, horizonY);
      Vector2i p2(fieldColorImage.size_.x() - 1, horizonY);
      fieldColorImage.line(p1, p2, Color::RED);

      debug().sendImage(mount_ + "." + imageData_->identification + "_image", fieldColorImage);
    }
  }
}
