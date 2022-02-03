#pragma once

#include "Framework/Debug/ImageConverterInterface.h"
#include <memory>

/**
 * @brief PngConverter encapsulates the functionality to convert an raw image to png.
 */
struct PngConverter : public ImageConverterInterface
{
  /**
   * @brief PngConverter constructor for the png converter.
   */
  PngConverter();


  /**
   * @brief convert is used to convert an image into data
   * @param img the image to be converted
   * @param data the vector for the output
   */
  void convert(const Image& img, CVData& data) override;

private:
  class Impl;
  std::shared_ptr<Impl> pImpl_;
};
