#include "ColorConverter.hpp"

#include "Tools/Storage/Image.hpp"
#include "Tools/Storage/Image422.hpp"

ColorConverter::ColorConverter() {}

/**
 * convertToYCbCr takes a pointer to a RGBA or BGRA buffer and converts it into an Image
 * @param img the output image
 * @param data the raw data from the Webots camera object
 */
void ColorConverter::RGB2YCbCr(Image& dst, const Image& src)
{
  int i, j;
  // The origin of the magic number can be found here:
  // http://en.wikipedia.org/wiki/YUV
  Color* out = dst.data_;
  const uint8_t* data = (const uint8_t*)src.data_;
  for (i = 0; i < src.size_.y(); i++)
  {
    for (j = 0; j < src.size_.x(); j++, data += 3, out++)
    {
      out->y_ = 0.299 * data[0] + 0.587 * data[1] + 0.114 * data[2];
      out->cb_ = 0.492 * (data[2] - out->y_);
      out->cr_ = 0.877 * (data[0] - out->y_);
    }
  }
}

void ColorConverter::BGR2YCbCr(Image& dst, const Image& src)
{
  int i, j;
  // The origin of the magic number can be found here:
  // http://en.wikipedia.org/wiki/YUV
  Color* out = dst.data_;
  const uint8_t* data = (const uint8_t*)src.data_;
  for (i = 0; i < src.size_.y(); i++)
  {
    for (j = 0; j < src.size_.x(); j++, data += 3, out++)
    {
      out->y_ = 0.299 * data[2] + 0.587 * data[1] + 0.114 * data[0];
      out->cb_ = 0.492 * (data[0] - out->y_);
      out->cr_ = 0.877 * (data[2] - out->y_);
    }
  }
}

void ColorConverter::YCbCr2RGB(Image& dst, const Image& src)
{
  int i, j;
  const Color* data = src.data_;
  uint8_t* out = (uint8_t*)dst.data_;

  for (i = 0; i < src.size_.y(); i++)
  {
    for (j = 0; j < src.size_.x(); j++, out += 3, data++)
    {
      float R, G, B;

      R = data->y_ + 1.402 * (data->cr_ - 128);
      G = data->y_ - 0.34414 * (data->cb_ - 128) - 0.71414 * (data->cr_ - 128);
      B = data->y_ + 1.772 * (data->cb_ - 128);

      out[0] = (R > 255) ? 255 : R;
      out[1] = (G > 255) ? 255 : G;
      out[2] = (B > 255) ? 255 : B;
    }
  }
}

void ColorConverter::YCbCr2BGR(Image& dst, const Image& src)
{
  int i, j;
  const Color* data = src.data_;
  uint8_t* out = (uint8_t*)dst.data_;

  for (i = 0; i < src.size_.y(); i++)
  {

    for (j = 0; j < src.size_.x(); j++, out += 3, data++)
    {
      float R, G, B;

      R = data->y_ + 1.402 * (data->cr_ - 128);
      G = data->y_ - 0.34414 * (data->cb_ - 128) - 0.71414 * (data->cr_ - 128);
      B = data->y_ + 1.772 * (data->cb_ - 128);

      out[2] = (R > 255) ? 255 : R;
      out[1] = (G > 255) ? 255 : G;
      out[0] = (B > 255) ? 255 : B;
    }
  }
}

Color ColorConverter::RGB2YCbCr(unsigned int R, unsigned int G, unsigned int B)
{
  return Color(16 + 0.2567890625 * R + 0.49631640625 * G + 0.09790625 * B, 128 - 0.14822265625 * R - 0.2909921875 * G + 0.43921484375 * B, 128 + 0.43921484375* R - 0.3677890625 * G - 0.07142578125 * B);
}

YCbCr422 ColorConverter::ycbcr422FromColor(const Color& c)
{
  return YCbCr422(c.y_, c.cb_, c.y_, c.cr_);
}

Color ColorConverter::colorFromYCbCr422(const YCbCr422& ycbcr422)
{
  return Color(ycbcr422.y1_, ycbcr422.cb_, ycbcr422.cr_);
}
