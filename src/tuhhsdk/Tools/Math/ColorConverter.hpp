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
  static void YCbCr2RGB(std::uint8_t (&dst) [3], std::uint8_t y, std::uint8_t cb, std::uint8_t cr);
  static void YCbCr2RGB(std::uint8_t (&dst) [3], YCbCr422& src);
  static void YCbCr2BGR(Image& dst, const Image& src);
  static Color RGB2YCbCr(unsigned int R, unsigned int G, unsigned int B);
  static YCbCr422 ycbcr422FromColor(const Color& c);
  static Color colorFromYCbCr422(const YCbCr422& ycbcr422);
};
