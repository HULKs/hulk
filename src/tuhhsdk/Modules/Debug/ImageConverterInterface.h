#pragma once

#include "Modules/Debug/DebugData.h"

class ImageConverterInterface
{
public:
  ImageConverterInterface() {}
  ~ImageConverterInterface() {}

  virtual SharedCVData convert(const Image& img) = 0;
};
