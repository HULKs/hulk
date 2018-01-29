#pragma once

#include <memory>

#include "Modules/Debug/ImageConverterInterface.h"

struct JpegConverter : ImageConverterInterface
{
  JpegConverter();
  virtual SharedCVData convert(const Image& img);

private:
  class Impl;
  std::shared_ptr<Impl> pImpl_;
};
