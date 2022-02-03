#pragma once

#include "Tools/Math/Eigen.hpp"
#include "Tools/Math/Line.hpp"
#include "Tools/Math/Polygon.hpp"
#include "Tools/Math/Rectangle.hpp"
#include "Tools/Storage/Color.hpp"
#include <cstdint>
#include <cstring>
#include <stdexcept>
#include <utility>

class XPMImage;

// Uncomment this for range checks. This will slow down every image access.
// #define IMAGE_DEBUG

class Image
{
public:
  /**
   * @brief Image initializes an empty image
   */
  constexpr Image() = default;

  /**
   * @brief Image allocates memory for an image of the specified size
   * @param size the dimensions of the new image
   */
  explicit Image(const Vector2i& size)
    : size{size}
    , data{new Color[size.x() * size.y()]}
    , realSize_{size}
  {
  }

  /**
   * @brief Image allocates memory for an image of the specified size and sets color
   * @param size the dimensions of the new image
   * @param color of created image
   */
  Image(const Vector2i& size, const Color& color)
    : size{size}
    , data{new Color[size.x() * size.y()]}
    , realSize_{size}
  {
    for (int i = 0; i < size.x() * size.y(); i++)
    {
      data[i] = color;
    }
  }

  /**
   * @brief Image copies the data from other to the created object
   * @param other the copy source
   */
  Image(const Image& other)
    : size{other.size}
    , data{new Color[size.x() * size.y()]}
    , name{other.name}
    , realSize_{size}
  {
    std::memcpy(data, other.data, size.x() * size.y() * sizeof(Color));
  }

  /**
   * @brief ~Image frees the data of the image
   */
  ~Image()
  {
    delete[] data;
  }

  /**
   * @brief operator= copies an image
   * @param other the copy source
   * @return the copied object
   */
  constexpr Image& operator=(const Image& other)
  {
    if (this == &other)
    {
      return *this;
    }
    if ((realSize_.x() * realSize_.y()) < (other.size.x() * other.size.y()))
    {
      delete[] data;
      data = new Color[other.size.x() * other.size.y()];
      realSize_ = other.size;
    }
    name = other.name;
    size = other.size;
    std::memcpy(data, other.data, size.x() * size.y() * sizeof(Color));
    return *this;
  }

  /**
   * @brief resize sets the size of an image and preallocates memory
   * @param size the new size of the image
   */
  void resize(const Vector2i& size)
  {
    if ((realSize_.x() * realSize_.y()) < (size.x() * size.y()))
    {
      delete[] data;
      data = new Color[size.x() * size.y()];
      realSize_ = size;
    }
    this->size = size;
  }

  /**
   * @brief operator[] returns a reference to the color identified by a vector
   * @param coords a vector that identifies the desired position
   * @return a reference to the pixel data in the image
   */
  Color& operator[](const Vector2i& coords)
  {
#ifdef IMAGE_DEBUG
    if (!isInside(coords))
    {
      throw std::runtime_error("Tried to access image out of bounds with operator[]");
    }
#endif
    return data[coords.y() * size.x() + coords.x()];
  }

  /**
   * @brief operator[] returns a constant reference to the color identified by a vector
   * @param coords a vector that identifies the desired position
   * @return a constant reference to the pixel data in the image
   */
  const Color& operator[](const Vector2i& coords) const
  {
#ifdef IMAGE_DEBUG
    if (!isInside(coords))
    {
      throw std::runtime_error("Tried to access image out of bounds with operator[]");
    }
#endif
    return data[coords.y() * size.x() + coords.x()];
  }

  /**
   * @brief at returns a reference to the color identified by two coordinates
   * @param y the y coordinate (down) of the pixel
   * @param x the x coordinate (right) of the pixel
   * @return a reference to the pixel data in the image
   */
  Color& at(unsigned int y, unsigned int x)
  {
#ifdef IMAGE_DEBUG
    if (!isInside(y, x))
    {
      throw std::runtime_error("Tried to access image out of bounds with at");
    }
#endif
    return data[y * size.x() + x];
  }

  /**
   * @brief at returns a reference to the color identified by two coordinates
   * @param point the x, y coordinate of the pixel
   * @return a reference to the pixel data in the image
   */
  Color& at(const Vector2i& point)
  {
    return at(point.y(), point.x());
  }

  /**
   * @brief at returns a constant reference to the color identified by two coordinates
   * @param y the y coordinate (down) of the pixel
   * @param x the x coordinate (right) of the pixel
   * @return a constant reference to the pixel data in the image
   */
  const Color& at(unsigned int y, unsigned int x) const
  {
#ifdef IMAGE_DEBUG
    if (!isInside(y, x))
    {
      throw std::runtime_error("Tried to access image out of bounds with at");
    }
#endif
    return data[y * size.x() + x];
  }

  /**
   * @brief at returns a constant reference to the color identified by two coordinates
   * @param point the x, y coordinate of the pixel
   * @return a constant reference to the pixel data in the image
   */
  const Color& at(const Vector2i& point) const
  {
    return at(point.y(), point.x());
  }

  /**
   * @brief isInside checks if a given point is inside the image
   * @param coords a vector that identifies the point
   * @return true if the point is inside the image
   */
  bool isInside(const Vector2i& coords) const
  {
    return ((coords.x() >= 0) && (coords.y() >= 0) && (coords.x() < size.x()) &&
            (coords.y() < size.y()));
  }

  /**
   * @brief isInside checks if a given point is inside the image
   * @param y the y coordinate (down) of the pixel
   * @param x the x coordinate (right) of the pixel
   * @return true if the point is inside the image
   */
  bool isInside(int y, int x) const
  {
    return ((x >= 0) && (y >= 0) && (x < size.x()) && (y < size.y()));
  }

  /**
   * @brief line draws a line between to points
   * @param p1 start point
   * @param p2 end point
   * @param color the color of the line
   * @return false if the line lies outside of the image
   */
  bool drawLine(const Vector2i& p1, const Vector2i& p2, const Color& color);

  /**
   * @brief draws a line on the image
   *
   * This simply wraps the line function that takes two points as arguments.
   *
   * @param l line to draw
   * @param color the color of the line
   * @return false if the line lies outside of the image
   */
  bool drawLine(const Line<int>& l, const Color& color);

  /**
   * @brief rectangle draws a rectangle around a center point
   * @param center the center of the rectangle
   * @param width the width of the rectangle
   * @param height the height of the rectangle
   * @param color the color of the rectangle
   */
  void drawRectangle(const Vector2i& center, int width, int height, const Color& color);

  /**
   * @brief rectangle draws a rectangle by passing two points
   * @param p1 point that is going to be the upper left corner of the rectangle
   * @param p2 point that is going to be the lower right corner of the rectangle
   * @param color the color of the rectangle
   */
  void drawRectangle(const Vector2i& p1, const Vector2i& p2, const Color& color);

  /**
   * @brief rectangle draws a rectangle by passing a rectangle
   * @param r the rectangle to draw
   * @param color the color of the rectangle
   */
  void drawRectangle(const Rectangle<int>& r, const Color& color);

  /**
   * @brief circle draws a circle around a point
   * @param center the center point of the circle
   * @param radius the radius of the circle
   * @param color the color of the circle
   */
  void drawCircle(const Vector2i& center, int radius, const Color& color);
  /**
   * @brief ellipse draws a ellipse around a point with given axes
   * @param center the center point of the ellipse in pixel coordinates
   * @param axes the length of the ellipse axes in pixels ("~radius")
   * @param rotation the rotation of the ellipse in radians
   * @param color the color the ellipse is going to be drawn in
   * @param resolution increasing this improves the quality of the drawn shape
   *                   (high values will result in high computational cost)
   */
  void drawEllipse(const Vector2i& center, const Vector2i& axes, const float rotation,
                   const Color& color, const int resolution = 10);

  /**
   * @brief draws a colored cross to mark a point
   * @param center the center point of the cross
   * @param color the color of the cross
   * @return false if the cross lies outside of the image
   */
  bool drawCross(const Vector2i& center, const int& size, const Color& color);

  /**
   * @brief draws a histogram on top of the image
   * @param values the histogram values. The number of values correlates to the number of boxes
   * @param color the color of the boxes in the histogram
   * @param precision the floating point precision to display (0 for no values to draw)
   * @param maxValue [optional] Whether to use the max from the given values or your own
   */
  void drawHistogram(const std::vector<int>& values, const Color& color, unsigned int precision,
                     float maxValue = 0.f);

  /**
   * @brief draws a histogram on top of the image
   * @param values the histogram values. The number of values correlates to the number of boxes
   * @param color the color of the boxes in the histogram
   * @param precision the floating point precision to display (0 for no values to draw)
   * @param maxValue [optional] Whether to use the max from the given values or your own
   */

  void drawHistogram(const std::vector<float>& values, const Color& color, unsigned int precision,
                     float maxValue = 0);
  /**
   * @brief draws an image at the given position
   * @param image the image
   * @param position the upper left position where the image gets drawn
   * @return whether the drawing was successful or not
   */
  bool drawImage(const Image& image, const Eigen::Matrix<unsigned int, 2, 1>& position);

  /**
   * @brief draws a (partial) xpm image to the given position
   * @param image the image
   * @param position the upper left position where the (partial) image gets drawn
   * @param upperLeft upper left position of the image to define a image segment
   * @param lowerRight lower right position of the image to define a image segment
   * @return whether the drawing was successful or not
   */
  bool drawImage(const Image& image, const Eigen::Matrix<unsigned int, 2, 1>& position,
                 const Eigen::Matrix<unsigned int, 2, 1>& upperLeft,
                 const Eigen::Matrix<unsigned int, 2, 1>& lowerRight, const Color* color = NULL);
  /**
   * @brief takes string and draws it to a specified position
   * @param str the string to draw (multiline with '\n')
   * @param position the position where to draw
   * @param color font color
   * @return whether the writing was successful or not
   */
  bool drawText(const std::string& str, Vector2i position, const Color& color);

  /**
   * @brief drawPolygon draws polygon edges
   * @param polygon the polygon to draw
   * @param color the polygon color
   * @return whether the drawing of all edges was successful
   */
  bool drawPolygon(const Polygon<int>& polygon, const Color& color);

  /// the dimensions of the image
  Vector2i size{0, 0};
  /// the image data, saved row by row
  Color* data{};
  /// the image name, e.g. full path in replay
  std::string name{};

private:
  /**
   * @brief clipLine clips a line to the image frame
   * @param pt1 start point of a line
   * @param pt2 end point of a line
   * @return false if the line lies oustide of the image
   */
  bool clipLine(Vector2i& pt1, Vector2i& pt2);
  /// the size for which memory is allocated
  Vector2i realSize_{0, 0};
};
