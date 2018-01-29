#pragma once

#include "Tools/Storage/Image.hpp"

#define RGB

class ColorConverter
{
public:
  ColorConverter();
  static void RGB2YCbCr(Image& dst, const Image& src);
  static void BGR2YCbCr(Image& dst, const Image& src);
  static void YCbCr2RGB(Image& dst, const Image& src);
  static void YCbCr2BGR(Image& dst, const Image& src);
  static Color RGB2YCbCr(unsigned int R, unsigned int G, unsigned int B);
};
