#include "Tools/Chronometer.hpp"
#include "Tools/Storage/Image.hpp"

#include "Vision/FieldColorDetection/ChromaticityFieldColorDetection.hpp"

ChromaticityFieldColorDetection::ChromaticityFieldColorDetection(
    const ModuleManagerInterface& manager)
  : Module(manager)
  , imageData_(*this)
  , cameraMatrix_(*this)
  , redChromaticityThreshold_(*this, "redChromaticityThreshold", [] {})
  , lowerGreenChromaticityThreshold_(*this, "lowerGreenChromaticityThreshold", [] {})
  , upperGreenChromaticityThreshold_(*this, "upperGreenChromaticityThreshold", [] {})
  , blueChromaticityThreshold_(*this, "blueChromaticityThreshold", [] {})
  , fieldColor_(*this)
{
}

void ChromaticityFieldColorDetection::setIsFieldColorFunctionForTop()
{
  fieldColor_->isFieldColor = [this](const YCbCr422& pixel) -> float {
    const auto rgbValues = RGBColor{pixel};
    const float redChromaticity = rgbValues.getChromaticity(rgbValues.r);
    const float greenChromaticity = rgbValues.getChromaticity(rgbValues.g);
    const float blueChromaticity = rgbValues.getChromaticity(rgbValues.b);
    if (redChromaticity < redChromaticityThreshold_()[0] &&
        blueChromaticity < blueChromaticityThreshold_()[0])
    {
      if (greenChromaticity > upperGreenChromaticityThreshold_()[0])
      {
        return 1.f;
      }
      if (greenChromaticity > lowerGreenChromaticityThreshold_()[0])
      {
        return 0.5f;
      }
    }
    return 0.f;
  };
}


void ChromaticityFieldColorDetection::setIsFieldColorFunctionForBottom()
{
  fieldColor_->isFieldColor = [this](const YCbCr422& pixel) -> float {
    const auto rgbValues = RGBColor{pixel};
    const float redChromaticity = rgbValues.getChromaticity(rgbValues.r);
    const float greenChromaticity = rgbValues.getChromaticity(rgbValues.g);
    const float blueChromaticity = rgbValues.getChromaticity(rgbValues.b);
    if (redChromaticity < redChromaticityThreshold_()[1] &&
        blueChromaticity < blueChromaticityThreshold_()[1])
    {
      if (greenChromaticity > upperGreenChromaticityThreshold_()[1])
      {
        return 1.f;
      }
      if (greenChromaticity > lowerGreenChromaticityThreshold_()[1])
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

    if (imageData_->cameraPosition == CameraPosition::TOP)
    {
      setIsFieldColorFunctionForTop();
    }
    else
    {
      setIsFieldColorFunctionForBottom();
    }

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
      for (int y = horizonY; y < fieldColorImage.size.y(); y += 2)
      {
        for (int x = 0; x < fieldColorImage.size.x(); ++x)
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
      Vector2i p2(fieldColorImage.size.x() - 1, horizonY);
      fieldColorImage.drawLine(p1, p2, Color::RED);

      debug().sendImage(mount_ + "." + imageData_->identification + "_image", fieldColorImage);
    }
  }
}
