#pragma once

#include "Tools/Storage/Image.hpp"
#include "Tools/Storage/Image422.hpp"

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
  static YCbCr422 ycbcr422FromColor(const Color& c);
  static Color colorFromYCbCr422(const YCbCr422& ycbcr422);
};
