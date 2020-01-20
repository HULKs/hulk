#pragma once

#include "Framework/Module.hpp"
#include "Tools/Storage/Image.hpp"
#include "Tools/Storage/UniValue/UniValue.h"

#include "Data/CameraMatrix.hpp"
#include "Data/FieldColor.hpp"
#include "Data/ImageData.hpp"

class Brain;

/**
 * @brief The FieldColorDetection class
 * To check whether a pixel is displaying a part of the field - i.e. is field color - thresholds in
 * the chromaticity color space are used. The red, green and blue chromaticity describe how green,
 * blue and red a pixel is respectively with no regard to the lightness of the pixel.
 */
class ChromaticityFieldColorDetection : public Module<ChromaticityFieldColorDetection, Brain>
{
public:
  /// the name of this module
  ModuleName name = "ChromaticityFieldColorDetection";
  explicit ChromaticityFieldColorDetection(const ModuleManagerInterface& manager);
  void cycle() override;

private:
  /// debug image counter to only send every third image
  unsigned int debugImageCounter_ = 0;

  /// the image that is currently being processed
  const Dependency<ImageData> imageData_;
  /// a reference to the camera matrix
  const Dependency<CameraMatrix> cameraMatrix_;
  /// field color must have less red chromaticity as redChromaticityThreshold_
  const Parameter<float> redChromaticityThreshold_;
  /// field color must have greater green chromaticity as this (0.5 sure)
  const Parameter<float> lowerGreenChromaticityThreshold_;
  /// field color must have greater green chromaticity as this to be 100 percent sure field color
  const Parameter<float> upperGreenChromaticityThreshold_;
  /// field color must have less blue chromaticity as blueChromaticityThreshold_
  const Parameter<float> blueChromaticityThreshold_;
  /// produces the isFieldColor function of FieldColor DataType
  void setIsFieldColorFunction();
  /// Sends debug image and results of (cb,cr)
  void sendImageForDebug(const Image422& image);

  /// the result of the field color detection
  Production<FieldColor> fieldColor_;
};
