#include <assert.h>
#include <cmath>
#include <stdexcept>

#include "Image.hpp"
#include "XPMImage.hpp"
#include "print.h"

const Color Color::RED(76, 84, 255);
const Color Color::GREEN(149, 43, 21);
const Color Color::BLUE(29, 255, 107);
const Color Color::WHITE(255, 128, 128);
const Color Color::BLACK(0, 128, 128);
const Color Color::ORANGE(151, 42, 201);
const Color Color::PINK(90, 147, 245);
const Color Color::TRANSPARENT(0, 0, 0);

bool Image::line(const Vector2i& p1, const Vector2i& p2, const Color& color)
{
  // more or less from wikipedia, Bresenham algorithm
  int dx, sx, dy, sy, err, e2, x, y;
  Vector2i pt1 = p1, pt2 = p2;
  if (!clipLine(pt1, pt2))
  {
    // The line lies completely outside of the image, return false;
    return false;
  }
  x = pt1.x();
  y = pt1.y();
  dx = std::abs(pt2.x() - x);
  dy = -std::abs(pt2.y() - y);
  sx = (x < pt2.x()) ? 1 : -1;
  sy = (y < pt2.y()) ? 1 : -1;
  err = dx + dy;
  while (true)
  {
    data_[y * size_.x() + x] = color;
    if ((x == pt2.x()) && (y == pt2.y()))
    {
      break;
    }
    e2 = 2 * err;
    if (e2 > dy)
    {
      err += dy;
      x += sx;
    }
    if (e2 < dx)
    {
      err += dx;
      y += sy;
    }
  }
  return true;
}

bool Image::line(const Line<int>& l, const Color& color)
{
  return line(l.p1, l.p2, color);
}

void Image::rectangle(const Vector2i& center, int width, int height, const Color& color)
{
  // pt1 is upper left corner, pt2 is upper right corner, pt3 is lower left corner and pt4 is lower right corner
  Vector2i pt1, pt2, pt3, pt4;
  pt1.x() = center.x() - width / 2;
  pt1.y() = center.y() - height / 2;
  pt2.x() = pt1.x() + width;
  pt2.y() = pt1.y();
  pt3.x() = pt1.x();
  pt3.y() = pt1.y() + height;
  pt4.x() = pt1.x() + width;
  pt4.y() = pt1.y() + height;

  // draw lines between pt1,pt2,pt3,pt4
  Image::line(pt1, pt2, color);
  Image::line(pt1, pt3, color);
  Image::line(pt2, pt4, color);
  Image::line(pt3, pt4, color);
}

void Image::rectangle(const Vector2i& p1, const Vector2i& p2, const Color& color)
{
  // pt1 is upper left corner, pt2 is upper right corner, pt3 is lower left corner and pt4 is lower right corner
  Vector2i pt1 = p1, pt2, pt3, pt4 = p2;
  pt2.x() = pt4.x();
  pt2.y() = pt1.y();
  pt3.x() = pt1.x();
  pt3.y() = pt4.y();

  // draw lines between pt1,pt2,pt3,pt4
  Image::line(pt1, pt2, color);
  Image::line(pt1, pt3, color);
  Image::line(pt2, pt4, color);
  Image::line(pt3, pt4, color);
}

// x_ has to be called x_ because x would interfer with size_.x
#define SET_PIXEL_CHECKED(y, x_)         \
  if (isInside((y), (x_)))               \
  {                                      \
    data_[(y)*size_.x() + (x_)] = color; \
  }

void Image::circle(const Vector2i& center, int radius, const Color& color)
{
  // more or less from wikipedia, Bresenham algorithm for circles
  int f, x, y, ddF_x, ddF_y;
  f = 1 - radius;
  x = 0;
  y = radius;
  ddF_x = 0;
  ddF_y = -2 * radius;
  // A circle can have parts outside the image even if the center is inside.
  // So every pixel has to be checked.
  SET_PIXEL_CHECKED(center.y() + radius, center.x());
  SET_PIXEL_CHECKED(center.y() - radius, center.x());
  SET_PIXEL_CHECKED(center.y(), center.x() + radius);
  SET_PIXEL_CHECKED(center.y(), center.x() - radius);
  while (x < y)
  {
    if (f >= 0)
    {
      y--;
      ddF_y += 2;
      f += ddF_y;
    }
    x++;
    ddF_x += 2;
    f += ddF_x + 1;
    SET_PIXEL_CHECKED(center.y() + y, center.x() + x);
    SET_PIXEL_CHECKED(center.y() + y, center.x() - x);
    SET_PIXEL_CHECKED(center.y() - y, center.x() + x);
    SET_PIXEL_CHECKED(center.y() - y, center.x() - x);
    SET_PIXEL_CHECKED(center.y() + x, center.x() + y);
    SET_PIXEL_CHECKED(center.y() + x, center.x() - y);
    SET_PIXEL_CHECKED(center.y() - x, center.x() + y);
    SET_PIXEL_CHECKED(center.y() - x, center.x() - y);
  }
}

//--------------- Cohen-Sutherland clipping algorithm -------------------------
// More or less from wikipedia
// Look here for explanation:
// https://en.wikipedia.org/wiki/Cohen-Sutherland_algorithm

// This function is only necessary for the line clipping function
int ComputeOutCode(Vector2i p, const Vector2i& size)
{
  int code;

  code = 0; // initialised as being inside of clip window

  if (p.x() < 0) // to the left of clip window
    code |= 1;
  else if (p.x() >= size.x()) // to the right of clip window
    code |= 2;
  if (p.y() < 0) // below the clip window
    code |= 4;
  else if (p.y() >= size.y()) // above the clip window
    code |= 8;

  return code;
}

bool Image::clipLine(Vector2i& p0, Vector2i& p1)
{
  int outcode0 = ComputeOutCode(p0, size_);
  int outcode1 = ComputeOutCode(p1, size_);

  while (true)
  {
    if (!(outcode0 | outcode1))
    {
      return true;
    }
    else if (outcode0 & outcode1)
    {
      break;
    }
    else
    {
      double x = 0, y = 0;
      int outcodeOut = outcode0 ? outcode0 : outcode1;

      if (outcodeOut & 8)
      { // point is above the clip rectangle
        x = p0.x() + (p1.x() - p0.x()) * (size_.y() - 1 - p0.y()) / (p1.y() - p0.y());
        y = size_.y() - 1;
      }
      else if (outcodeOut & 4)
      { // point is below the clip rectangle
        x = p0.x() + (p1.x() - p0.x()) * (0 - p0.y()) / (p1.y() - p0.y());
        y = 0;
      }
      else if (outcodeOut & 2)
      { // point is to the right of clip rectangle
        y = p0.y() + (p1.y() - p0.y()) * (size_.x() - 1 - p0.x()) / (p1.x() - p0.x());
        x = size_.x() - 1;
      }
      else if (outcodeOut & 1)
      { // point is to the left of clip rectangle
        y = p0.y() + (p1.y() - p0.y()) * (0 - p0.x()) / (p1.x() - p0.x());
        x = 0;
      }

      if (outcodeOut == outcode0)
      {
        p0.x() = x;
        p0.y() = y;
        outcode0 = ComputeOutCode(p0, size_);
      }
      else
      {
        p1.x() = x;
        p1.y() = y;
        outcode1 = ComputeOutCode(p1, size_);
      }
    }
  }
  return false;
}

//----------------------- Clipping algorithm end ------------------------------

bool Image::cross(const Vector2i& center, const int& size, const Color& color)
{
  Vector2i p_top(center.x(), center.y() - size);
  Vector2i p_bottom(center.x(), center.y() + size);
  Vector2i p_left(center.x() - size, center.y());
  Vector2i p_right(center.x() + size, center.y());

  // Using the |-operator on purpose here because || would sometimes cause a unwanted short-circuit evaluation.
  // Only if both lines failed to be drawn, the cross isn't visible at all
  if (!(line(p_top, p_bottom, color) | line(p_left, p_right, color)))
  {
    // Cross lies oustide of the image
    return false;
  }
  else
  {
    return true;
  }
}

bool Image::drawImage(const Image& image, const Eigen::Matrix<unsigned int, 2, 1>& position)
{
  return drawImage(image, position, Eigen::Matrix<unsigned int, 2, 1>(0, 0), Eigen::Matrix<unsigned int, 2, 1>(image.size_.x() - 1, image.size_.y() - 1));
}

bool Image::drawImage(const Image& image, const Eigen::Matrix<unsigned int, 2, 1>& position, const Eigen::Matrix<unsigned int, 2, 1>& upperLeft, const Eigen::Matrix<unsigned int, 2, 1>& lowerRight, const Color* color)
{
  if (!(upperLeft.x() < static_cast<unsigned int>(image.size_.x()) && upperLeft.y() < static_cast<unsigned int>(image.size_.y())) ||
    !(lowerRight.x() < static_cast<unsigned int>(image.size_.x()) && lowerRight.y() < static_cast<unsigned int>(image.size_.y())) ||
    !(upperLeft.x() <= lowerRight.x() && upperLeft.y() <= lowerRight.y()))
  {
    assert(false);
    return false;
  }
  unsigned int xdiff = lowerRight.x() - upperLeft.x();
  unsigned int ydiff = lowerRight.y() - upperLeft.y();
  for (unsigned int y = position.y(); y < (position.y() + ydiff) && y < static_cast<unsigned int>(size_.y()); ++y)
  {
    for (unsigned int x = position.x(); x < (position.x() + xdiff) && x < static_cast<unsigned int>(size_.x()); ++x)
    {
      const Color pixel = image.data_[(y - position.y() + upperLeft.y()) * image.size_.x() + (x - position.x() + upperLeft.x())];
      if (pixel == Color::TRANSPARENT)
      {
        continue;
      }
      data_[y * size_.x() + x] = color ? *color : pixel;
    }
  }
  return true;
}

bool Image::drawString(const std::string& str, const Vector2i& position, const Color& color)
{
  const char *cStr = str.c_str();
  for (unsigned int i = 0; i < str.size(); ++i)
  {
    const char c = cStr[i];
    unsigned int y = c / 16;
    unsigned int x = c - 16 * y;
    Eigen::Matrix<unsigned int, 2, 1> upperLeft = Eigen::Matrix<unsigned int, 2, 1>(x * 16, y * 16);
    Image::drawImage(XPMImage::ascii16x16_, Eigen::Matrix<unsigned int, 2, 1>(position.x() + i * 16, position.y()), upperLeft, Eigen::Matrix<unsigned int, 2, 1>(upperLeft.x() + 16, upperLeft.y() + 16), &color);
  }
  return true;
}
