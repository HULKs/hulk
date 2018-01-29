#pragma once

#include "Modules/Debug/ImageConverterInterface.h"
#include <memory>

struct PngConverter : ImageConverterInterface
{
  PngConverter();
  virtual SharedCVData convert(const Image& img);

private:
  class Impl;
  std::shared_ptr<Impl> pImpl_;
};
