#pragma once

#include "Framework/Module.hpp"
#include "Tools/Storage/Image.hpp"
#include "Tools/Storage/UniValue/UniValue.h"

#include "Data/CameraMatrix.hpp"
#include "Data/FieldColor.hpp"
#include "Data/ImageData.hpp"

class Brain;

/**
 * @brief The SaturationImageProvider produces a debug image to visualize saturated pixels
 */
class SaturationImageProvider : public Module<SaturationImageProvider, Brain>
{
public:
  /// the name of this module
  ModuleName name__{"SaturationImageProvider"};
  SaturationImageProvider(const ModuleManagerInterface& manager);
  void cycle();

private:
  /// the image that is currently being processed
  const Dependency<ImageData> imageData_;
  /// debug image counter
  unsigned int counter_;
  /// Sends debug image
  void sendImageForDebug(const Image422& image);
};
