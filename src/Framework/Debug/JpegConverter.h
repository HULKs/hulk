#pragma once

#include <memory>

#include "Framework/Debug/ImageConverterInterface.h"

struct JpegConverter : public ImageConverterInterface
{
  JpegConverter();
  void convert(const Image& img, CVData& data);

private:
  class Impl;
  std::shared_ptr<Impl> pImpl_;
};
