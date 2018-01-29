#include "Definitions/XPMImages.h"
#include "Definitions/X11rgb.h"

#include "XPMImage.hpp"

std::map<std::string, Color> XPMImage::x11Colors_;
XPMImage XPMImage::ascii16x16_;

XPMImage XPMImage::loadXPMImage(const char **rawXPM, const char* transparencyChar)
{
  std::regex metaRegEx{R"foo( *(\d{1,}) (\d{1,}) (\d{1,}) (\d{1,}) *)foo"};
  std::string meta(rawXPM[0]);
  std::smatch metadata;
  if (!std::regex_match(meta, metadata, metaRegEx))
  {
    assert(false);
  }
  unsigned int width = std::stoi(metadata[1].str());
  unsigned int height = std::stoi(metadata[2].str());
  unsigned int numberOfColors = std::stoi(metadata[3].str());
  unsigned int charPerPixel = std::stoi(metadata[4].str());

  std::map<std::string, Color> colorMap;
  std::regex colorMapRegEx{R"foo((.*)\s[c]\s(\w{1,}|#{1}\w{1,6}))foo"};
  for (unsigned int i = 0; i < numberOfColors; ++i)
  {
    std::string color(rawXPM[i + 1]);
    std::smatch colorPair;
    if (!std::regex_match(color, colorPair, colorMapRegEx))
    {
      assert(false);
    }
    if (colorPair[2].str().compare(0,1,"#") == 0) // check for hex value
    {
      assert(colorPair[2].str().size() == 7);
      unsigned int rgb[3];
      unsigned int v = 0;
      // parse rgb hex value to ycbcr dec color
      for (unsigned int i = 1; i < 6; i += 2)
      {
        char hexValue[3];
        memcpy(hexValue, &colorPair[2].str()[i], 2 * sizeof(char));
        hexValue[2] = '\0';
        std::stringstream ss;
        ss << hexValue;
        ss >> std::hex >> rgb[v];
        ++v;
      }
      colorMap[colorPair[1].str()] = ColorConverter::RGB2YCbCr(rgb[0], rgb[1], rgb[2]);
    }
    else // x11 color
    {
      colorMap[colorPair[1].str()] = x11Colors_[colorPair[2]];
    }
  }

  XPMImage image(Vector2i(width, height));

  for (unsigned int y = 0; y < height; ++y)
  {
    for (unsigned int x = 0; x < width; ++x)
    {
      assert(charPerPixel < 5);
      char subStr[5];
      memcpy(subStr, &rawXPM[y + 1 + numberOfColors][x * charPerPixel], charPerPixel * sizeof(char));
      subStr[charPerPixel] = '\0';
      if (transparencyChar && *subStr == *transparencyChar)
      {
        image.data_[y * width + x] = Color::TRANSPARENT;
      }
      else
      {
        image.data_[y * width + x] = colorMap[subStr];
      }
    }
  }

  return image;
}

void XPMImage::init()
{
  // load x11 colors
  std::regex colorX11RegEx{R"foo(\s*(\d{1,3})\s*(\d{1,3})\s*(\d{1,3})\s*((?:\w{0,} *)*))foo"};
  for (unsigned int i = 0; i < sizeof(x11RGB)/8; ++i)
  {
    std::string row(x11RGB[i]);
    std::smatch colorValues;
    if (!std::regex_match(row, colorValues, colorX11RegEx))
    {
      assert(false);
    }
    x11Colors_[colorValues[4].str()] = ColorConverter::RGB2YCbCr(std::stoi(colorValues[1].str()), std::stoi(colorValues[2].str()), std::stoi(colorValues[3].str()));
  }

  // xpm images
  const char ascii16x16TransparencyChar = 32;
  ascii16x16_ = loadXPMImage(ascii16x16, &ascii16x16TransparencyChar);
}

