#include <algorithm>
#include <assert.h>
#include <cmath>
#include <iomanip>
#include <stdexcept>

#include "Image.hpp"
#include "XPMImage.hpp"
#include "print.h"

const Color Color::RED(76, 84, 255);
const Color Color::GREEN(149, 43, 21);
const Color Color::BLUE(29, 255, 107);
const Color Color::WHITE(255, 128, 128);
const Color Color::BLACK(0, 128, 128);
const Color Color::YELLOW(208, 16, 146);
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
  // pt1 is upper left corner, pt2 is upper right corner, pt3 is lower left corner and pt4 is lower
  // right corner
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
  // pt1 is upper left corner, pt2 is upper right corner, pt3 is lower left corner and pt4 is lower
  // right corner
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

void Image::rectangle(const Rectangle<int>& r, const Color& color)
{
  Image::rectangle(r.topLeft, r.bottomRight, color);
}

// x_ has to be called x_ because x would interfer with size_.x
#define SET_PIXEL_CHECKED(y, x_)                                                                   \
  if (isInside((y), (x_)))                                                                         \
  {                                                                                                \
    data_[(y)*size_.x() + (x_)] = color;                                                           \
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
int ComputeOutCode(const Vector2i& p, const Vector2i& size)
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

void Image::ellipse(const Vector2i& center, const Vector2i& axes, const float rotation,
                    const Color& color, const int resolution)
{
  // TODO: use Bresenham algorithm
  // x' = a*cos(t)*cos(theta) - b*sin(t)*sin(theta)
  // y' = a*cos(t)*sin(theta) + b*sin(t)*cos(theta)

  Vector2<int> start_point, current_point, last_point;
  float ctheta = std::cos(rotation), stheta = std::sin(rotation);
  start_point.x() = static_cast<int>(axes.x() * std::cos(0) * ctheta -
                                     axes.y() * stheta * std::sin(0) + center.x());
  start_point.y() = static_cast<int>(axes.x() * std::cos(0) * stheta -
                                     axes.y() * ctheta * std::sin(0) + center.y());

  last_point = start_point;

  for (int i = 1; i < resolution; ++i)
  {
    double t = 2 * M_PI / resolution * i;
    current_point.x() = static_cast<int>(axes.x() * std::cos(t) * ctheta -
                                         axes.y() * stheta * std::sin(t) + center.x());
    current_point.y() = static_cast<int>(axes.x() * std::cos(t) * stheta -
                                         axes.y() * ctheta * std::sin(t) + center.y());

    line(last_point, current_point, color);
    last_point = current_point;
  }

  line(last_point, start_point, color);
}

//----------------------- Clipping algorithm end ------------------------------

bool Image::cross(const Vector2i& center, const int& size, const Color& color)
{
  Vector2i p_top(center.x(), center.y() - size);
  Vector2i p_bottom(center.x(), center.y() + size);
  Vector2i p_left(center.x() - size, center.y());
  Vector2i p_right(center.x() + size, center.y());

  // Using the |-operator on purpose here because || would sometimes cause a unwanted short-circuit
  // evaluation. Only if both lines failed to be drawn, the cross isn't visible at all
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

void Image::histogram(const std::vector<int>& values, const Color& color, unsigned int precision,
                      float maxValue)
{
  std::vector<float> floatValues(values.begin(), values.end());
  return histogram(floatValues, color, precision, maxValue);
}

void Image::histogram(const std::vector<float>& values, const Color& color, unsigned int precision,
                      float maxValue)
{
  // Max drawing height
  const unsigned int minPixelY = 0.2f * size_.y();
  // Useable space in y direction to draw a box
  const unsigned int maxPixelY = size_.y() - minPixelY;
  // If no maxValue is given it defaults to zero.
  if (!maxValue)
  {
    // Determine max value on my own
    const auto elementIt = std::max_element(values.begin(), values.end());
    const unsigned int pos = elementIt - values.begin();
    maxValue = values[pos];
  }
  // Scale factor
  const float factor = maxPixelY / maxValue;
  // Box width
  const unsigned int boxWidth = size_.x() / values.size();
  // Give me a little room
  const int safetyDistance = 5;
  const int fontSize = 16;
  const int markerLength = 20;
  // Offset to draw string at correct position
  const int offset = fontSize + safetyDistance;
  // String to indicate max value on the top left
  drawString(std::to_string(maxValue), Vector2i(safetyDistance, minPixelY - offset), Color::RED);
  // Small marker on the left and right to indicate max value
  line(Vector2i(0, minPixelY), Vector2i(markerLength, minPixelY), Color::RED);
  line(Vector2i(size_.x(), minPixelY), Vector2i(size_.x() - markerLength, minPixelY), Color::RED);
  for (unsigned int i = 0; i < values.size(); ++i)
  {
    // Draw box
    rectangle(Vector2i(i * boxWidth,
                       size_.y() - std::min(values[i] * factor, static_cast<float>(maxPixelY))),
              Vector2i((i + 1) * boxWidth, size_.y()), color);
    if (!precision)
    {
      continue;
    }
    // Draw its value
    std::stringstream shortValue;
    shortValue << std::setprecision(precision) << values[i];
    drawString(shortValue.str(), Vector2i(i * boxWidth + safetyDistance, size_.y() - offset),
               Color::BLACK);
  }
}

bool Image::drawImage(const Image& image, const Eigen::Matrix<unsigned int, 2, 1>& position)
{
  return drawImage(image, position, Eigen::Matrix<unsigned int, 2, 1>(0, 0),
                   Eigen::Matrix<unsigned int, 2, 1>(image.size_.x() - 1, image.size_.y() - 1));
}

bool Image::drawImage(const Image& image, const Eigen::Matrix<unsigned int, 2, 1>& position,
                      const Eigen::Matrix<unsigned int, 2, 1>& upperLeft,
                      const Eigen::Matrix<unsigned int, 2, 1>& lowerRight, const Color* color)
{
  if (!(upperLeft.x() < static_cast<unsigned int>(image.size_.x()) &&
        upperLeft.y() < static_cast<unsigned int>(image.size_.y())) ||
      !(lowerRight.x() < static_cast<unsigned int>(image.size_.x()) &&
        lowerRight.y() < static_cast<unsigned int>(image.size_.y())) ||
      !(upperLeft.x() <= lowerRight.x() && upperLeft.y() <= lowerRight.y()))
  {
    assert(false);
    return false;
  }
  unsigned int xdiff = lowerRight.x() - upperLeft.x();
  unsigned int ydiff = lowerRight.y() - upperLeft.y();
  for (unsigned int y = position.y();
       y < (position.y() + ydiff) && y < static_cast<unsigned int>(size_.y()); ++y)
  {
    for (unsigned int x = position.x();
         x < (position.x() + xdiff) && x < static_cast<unsigned int>(size_.x()); ++x)
    {
      const Color pixel = image.data_[(y - position.y() + upperLeft.y()) * image.size_.x() +
                                      (x - position.x() + upperLeft.x())];
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
  const char* cStr = str.c_str();
  for (unsigned int i = 0; i < str.size(); ++i)
  {
    const char c = cStr[i];
    unsigned int y = c / 16;
    unsigned int x = c - 16 * y;
    Eigen::Matrix<unsigned int, 2, 1> upperLeft = Eigen::Matrix<unsigned int, 2, 1>(x * 16, y * 16);
    Image::drawImage(
        XPMImage::ascii16x16_,
        Eigen::Matrix<unsigned int, 2, 1>(position.x() + i * 16, position.y()), upperLeft,
        Eigen::Matrix<unsigned int, 2, 1>(upperLeft.x() + 16, upperLeft.y() + 16), &color);
  }
  return true;
}

bool Image::drawPolygon(const Polygon<int>& polygon, const Color& color)
{
  bool status = true;
  for (unsigned int i = 0, j = polygon.points.size() - 1; i < polygon.points.size(); j = i, ++i)
  {
    status = status && Image::line(polygon.points[i], polygon.points[j], color);
  }
  return status;
}
