#pragma once

#include "Modules/Debug/DebugData.h"

class ImageConverterInterface
{
public:
  ImageConverterInterface() {}
  virtual ~ImageConverterInterface() {}

  virtual void convert(const Image& img, CVData& data) = 0;
};
