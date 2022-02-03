#ifndef XPMIMAGE_HPP
#define XPMIMAGE_HPP

#include <iostream>
#include <map>
#include <regex>
#include <string>

#include "Tools/Storage/Image.hpp"


class Image;

class XPMImage : public Image
{

public:
  static XPMImage loadXPMImage(const char** rawXPM, const char* transparencyChar = nullptr);
  static void init();
  static std::map<std::string, Color> x11Colors_;
  static XPMImage ascii16x16_;

private:
  XPMImage()
    : Image()
  {
  }

  XPMImage(const Vector2<int>& size)
    : Image(size)
  {
  }
};

#endif // XPMIMAGE_HPP
