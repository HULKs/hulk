#pragma once

#include "Data/ImageData.hpp"
#include "Framework/Module.hpp"

class Brain;

/**
 * @brief Provides the debug images for multiple color spaces
 */
class ColorSpaceImagesProvider : public Module<ColorSpaceImagesProvider, Brain>
{
public:
  /// the name of this module
  ModuleName name__{"ColorSpaceImagesProvider"};
  /**
   *@brief The constructor of this class
   */
  explicit ColorSpaceImagesProvider(const ModuleManagerInterface& manager);

  void cycle() override;

private:
  const Dependency<ImageData> imageData_;

  /*
   * @brief sends various colorspace images via debug
   * @param image reference to the image of which this debug image is build
   */
  void sendImagesForDebug(const Image422& image) const;

  /*
   * @brief sends a variable generated grayscale image via debug
   * @param image reference to the image of which this debug image is build
   * @param name unique name identifying this image in debug mounts
   * @param getValue A function taking a color object of the referenced image and generating the one
   * dimensional output
   */
  void sendGrayscaleImage(const Image422& image, const std::string& name,
                          const std::function<std::uint8_t(const Color&)>& getValue) const;
};
