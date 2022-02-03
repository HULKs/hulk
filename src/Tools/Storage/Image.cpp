#include "Tools/Storage/Image.hpp"
#include "Framework/Log/Log.hpp"
#include "Tools/Storage/XPM/XPMImage.hpp"
#include <algorithm>
#include <cassert>
#include <cmath>
#include <iomanip>
#include <stdexcept>

bool Image::drawLine(const Vector2i& p1, const Vector2i& p2, const Color& color)
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
    data[y * size.x() + x] = color;
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

bool Image::drawLine(const Line<int>& l, const Color& color)
{
  return drawLine(l.p1, l.p2, color);
}

void Image::drawRectangle(const Vector2i& center, int width, int height, const Color& color)
{
  // pt1 is upper left corner, pt2 is upper right corner, pt3 is lower left corner and pt4 is
  // lower right corner
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
  Image::drawLine(pt1, pt2, color);
  Image::drawLine(pt1, pt3, color);
  Image::drawLine(pt2, pt4, color);
  Image::drawLine(pt3, pt4, color);
}

void Image::drawRectangle(const Vector2i& p1, const Vector2i& p2, const Color& color)
{
  // pt1 is upper left corner, pt2 is upper right corner, pt3 is lower left corner and pt4 is
  // lower right corner
  Vector2i pt1 = p1, pt2, pt3, pt4 = p2;
  pt2.x() = pt4.x();
  pt2.y() = pt1.y();
  pt3.x() = pt1.x();
  pt3.y() = pt4.y();

  // draw lines between pt1,pt2,pt3,pt4
  Image::drawLine(pt1, pt2, color);
  Image::drawLine(pt1, pt3, color);
  Image::drawLine(pt2, pt4, color);
  Image::drawLine(pt3, pt4, color);
}

void Image::drawRectangle(const Rectangle<int>& r, const Color& color)
{
  Image::drawRectangle(r.topLeft, r.bottomRight, color);
}

// x_ has to be called x_ because x would interfer with size.x
#define SET_PIXEL_CHECKED(y, x_)                                                                   \
  if (isInside((y), (x_)))                                                                         \
  {                                                                                                \
    data[(y)*size.x() + (x_)] = color;                                                             \
  }

void Image::drawCircle(const Vector2i& center, int radius, const Color& color)
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
int computeOutCode(const Vector2i& p, const Vector2i& size)
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
  int outcode0 = computeOutCode(p0, size);
  int outcode1 = computeOutCode(p1, size);

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
        x = p0.x() + (p1.x() - p0.x()) * (size.y() - 1 - p0.y()) / (p1.y() - p0.y());
        y = size.y() - 1;
      }
      else if (outcodeOut & 4)
      { // point is below the clip rectangle
        x = p0.x() + (p1.x() - p0.x()) * (0 - p0.y()) / (p1.y() - p0.y());
        y = 0;
      }
      else if (outcodeOut & 2)
      { // point is to the right of clip rectangle
        y = p0.y() + (p1.y() - p0.y()) * (size.x() - 1 - p0.x()) / (p1.x() - p0.x());
        x = size.x() - 1;
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
        outcode0 = computeOutCode(p0, size);
      }
      else
      {
        p1.x() = x;
        p1.y() = y;
        outcode1 = computeOutCode(p1, size);
      }
    }
  }
  return false;
}

void Image::drawEllipse(const Vector2i& center, const Vector2i& axes, const float rotation,
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

    drawLine(last_point, current_point, color);
    last_point = current_point;
  }

  drawLine(last_point, start_point, color);
}

//----------------------- Clipping algorithm end ------------------------------

bool Image::drawCross(const Vector2i& center, const int& size, const Color& color)
{
  Vector2i p_top(center.x(), center.y() - size);
  Vector2i p_bottom(center.x(), center.y() + size);
  Vector2i p_left(center.x() - size, center.y());
  Vector2i p_right(center.x() + size, center.y());

  // Using the |-operator on purpose here because || would sometimes cause a unwanted
  // short-circuit evaluation. Only if both lines failed to be drawn, the cross isn't visible at
  // all
  if (!(drawLine(p_top, p_bottom, color) | drawLine(p_left, p_right, color)))
  {
    // Cross lies oustide of the image
    return false;
  }
  else
  {
    return true;
  }
}

void Image::drawHistogram(const std::vector<int>& values, const Color& color,
                          unsigned int precision, float maxValue)
{
  std::vector<float> floatValues(values.begin(), values.end());
  return drawHistogram(floatValues, color, precision, maxValue);
}

void Image::drawHistogram(const std::vector<float>& values, const Color& color,
                          unsigned int precision, float maxValue)
{
  // Max drawing height
  const unsigned int minPixelY = 0.2f * size.y();
  // Useable space in y direction to draw a box
  const unsigned int maxPixelY = size.y() - minPixelY;
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
  const unsigned int boxWidth = size.x() / values.size();
  // Give me a little room
  const int safetyDistance = 5;
  const int fontSize = 16;
  const int markerLength = 20;
  // Offset to draw string at correct position
  const int offset = fontSize + safetyDistance;
  // String to indicate max value on the top left
  drawText(std::to_string(maxValue), Vector2i(safetyDistance, minPixelY - offset), Color::RED);
  // Small marker on the left and right to indicate max value
  drawLine(Vector2i(0, minPixelY), Vector2i(markerLength, minPixelY), Color::RED);
  drawLine(Vector2i(size.x(), minPixelY), Vector2i(size.x() - markerLength, minPixelY), Color::RED);
  for (unsigned int i = 0; i < values.size(); ++i)
  {
    // Draw box
    drawRectangle(Vector2i(i * boxWidth,
                           size.y() - std::min(values[i] * factor, static_cast<float>(maxPixelY))),
                  Vector2i((i + 1) * boxWidth, size.y()), color);
    if (!precision)
    {
      continue;
    }
    // Draw its value
    std::stringstream shortValue;
    shortValue << std::setprecision(precision) << values[i];
    drawText(shortValue.str(), Vector2i(i * boxWidth + safetyDistance, size.y() - offset),
             Color::BLACK);
  }
}

bool Image::drawImage(const Image& image, const Eigen::Matrix<unsigned int, 2, 1>& position)
{
  return drawImage(image, position, Eigen::Matrix<unsigned int, 2, 1>(0, 0),
                   Eigen::Matrix<unsigned int, 2, 1>(image.size.x(), image.size.y()));
}

bool Image::drawImage(const Image& image, const Eigen::Matrix<unsigned int, 2, 1>& position,
                      const Eigen::Matrix<unsigned int, 2, 1>& upperLeft,
                      const Eigen::Matrix<unsigned int, 2, 1>& lowerRight, const Color* color)
{
  if (!(upperLeft.x() < static_cast<unsigned int>(image.size.x()) &&
        upperLeft.y() < static_cast<unsigned int>(image.size.y())) ||
      !(lowerRight.x() <= static_cast<unsigned int>(image.size.x()) &&
        lowerRight.y() <= static_cast<unsigned int>(image.size.y())) ||
      !(upperLeft.x() <= lowerRight.x() && upperLeft.y() <= lowerRight.y()))
  {
    assert(false);
    return false;
  }
  unsigned int xdiff = lowerRight.x() - upperLeft.x();
  unsigned int ydiff = lowerRight.y() - upperLeft.y();
  for (unsigned int y = position.y();
       y < (position.y() + ydiff) && y < static_cast<unsigned int>(size.y()); ++y)
  {
    for (unsigned int x = position.x();
         x < (position.x() + xdiff) && x < static_cast<unsigned int>(size.x()); ++x)
    {
      const Color pixel = image.data[(y - position.y() + upperLeft.y()) * image.size.x() +
                                     (x - position.x() + upperLeft.x())];
      if (pixel == Color::TRANSPARENT)
      {
        continue;
      }
      data[y * size.x() + x] = color ? *color : pixel;
    }
  }
  return true;
}

bool Image::drawText(const std::string& str, Vector2i position, const Color& color)
{
  // character advance in pixel of character sprite map XPMImage::ascii16x16_
  const std::size_t characterOffset = 16;

  // split string into lines (https://stackoverflow.com/a/59267272)
  std::vector<std::string> lines;
  std::stringstream ss{str};
  std::size_t width = 0;  // width of text bounding box
  std::size_t height = 0; // height of text bounding box
  std::string line;
  while (std::getline(ss, line, '\n'))
  {
    lines.emplace_back(line);
    width = std::max(width, line.size());
    height++;
  }

  // calculate string position by fitting text bounding box into image
  // remove negative position components
  position.x() = std::max(position.x(), 0);
  position.y() = std::max(position.y(), 0);
  // when the bounding box is larger than the image, left align
  if (width * characterOffset > static_cast<std::size_t>(size.x()))
  {
    position.x() = 0;
  }
  // when the bounding box goes beyond the image boundary, move left to fit
  else if (static_cast<std::size_t>(position.x()) + width * characterOffset >
           static_cast<std::size_t>(size.x()))
  {
    position.x() = size.x() - width * characterOffset;
  }
  // when the bounding box is larger than the image, top align
  if (height * characterOffset > static_cast<std::size_t>(size.y()))
  {
    position.y() = 0;
  }
  // when the bounding box goes beyond the image boundary, move top to fit
  else if (static_cast<std::size_t>(position.y()) + height * characterOffset >
           static_cast<std::size_t>(size.y()))
  {
    position.y() = size.y() - height * characterOffset;
  }

  // iterate over lines and characters and draw each character
  for (std::size_t y = 0; y < lines.size(); ++y)
  {
    auto& line = lines[y];
    for (std::size_t x = 0; x < line.size(); ++x)
    {
      // calculate character position (as indices) in character sprite map XPMImage::ascii16x16_
      std::size_t characterMapX = line[x] % characterOffset;
      std::size_t characterMapY = line[x] / characterOffset;
      // draw a character at position + (x, y) (advance by characterOffset)
      //   the upper left is the calculated character position in sprite map
      //   the lower right is the calculated character position + (1, 1) (advanced by
      //   characterOffset)
      drawImage(XPMImage::ascii16x16_,
                {position.x() + x * characterOffset, position.y() + y * characterOffset},
                {characterMapX * characterOffset, characterMapY * characterOffset},
                {(characterMapX + 1) * characterOffset, (characterMapY + 1) * characterOffset},
                &color);
    }
  }

  return true;
}

bool Image::drawPolygon(const Polygon<int>& polygon, const Color& color)
{
  bool status = true;
  for (unsigned int i = 0, j = polygon.points.size() - 1; i < polygon.points.size(); j = i, ++i)
  {
    status = status && Image::drawLine(polygon.points[i], polygon.points[j], color);
  }
  return status;
}
