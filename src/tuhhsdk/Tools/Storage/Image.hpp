#pragma once

#include <cstdint>
#include <cstring>
#include <stdexcept>

#include "Tools/Math/Eigen.hpp"
#include "Tools/Math/Line.hpp"
#include "Tools/Math/Polygon.hpp"
#include "Tools/Math/Rectangle.hpp"

class XPMImage;

// Uncomment this for range checks. This will slow down every image access.
// #define IMAGE_DEBUG

struct Color
{
  /**
   * @brief Color initializes the channels with 0
   * @author Arne Hasselbring
   */
  Color()
    : y_(0)
    , cb_(0)
    , cr_(0)
  {
  }
  /**
   * @brief Color initializes the channels with user-defines values
   * @param y the initial value for the y channel
   * @param cb the initial value for the cb channel
   * @param cr the initial value for the cr channel
   * @author Arne Hasselbring
   */
  Color(std::uint8_t y, std::uint8_t cb, std::uint8_t cr)
    : y_(y)
    , cb_(cb)
    , cr_(cr)
  {
  }
  /// y channel
  std::uint8_t y_;
  /// cb/u channel
  std::uint8_t cb_;
  /// cr/v channel
  std::uint8_t cr_;
  /// static member for red
  static const Color RED;
  /// static member for green
  static const Color GREEN;
  /// static member for blue
  static const Color BLUE;
  /// static member for white
  static const Color WHITE;
  /// static member for black
  static const Color BLACK;
  /// static member for yellow
  static const Color YELLOW;
  /// static member for orange
  static const Color ORANGE;
  /// static member for pink
  static const Color PINK;
  /// static member for transparency
  static const Color TRANSPARENT;

  /**
   * @brief Comparison with another color
   */
  bool operator==(const Color& other) const
  {
    return y_ == other.y_ && cb_ == other.cb_ && cr_ == other.cr_;
  }
};

class Image
{
public:
  /**
   * @brief Image initializes an empty image
   * @author Arne Hasselbring
   */
  Image()
    : size_(0, 0)
    , data_(NULL)
    , name_("")
    , real_size_(0, 0)
  {
  }

  /**
   * @brief Image allocates memory for an image of the specified size
   * @param size the dimensions of the new image
   * @author Arne Hasselbring
   */
  Image(const Vector2i& size)
    : size_(size)
    , data_(new Color[size_.x() * size_.y()])
    , name_("")
    , real_size_(size_)
  {
  }

  /**
   * @brief Image allocates memory for an image of the specified size and sets color
   * @param size the dimensions of the new image
   * @param color of created image
   * @author Felix Warmuth
   */
  Image(const Vector2i& size, const Color& color)
    : size_(size)
    , data_(new Color[size_.x() * size_.y()])
    , name_("")
    , real_size_(size_)
  {
    for (int i = 0; i < size_.x() * size_.y(); i++)
    {
      data_[i] = color;
    }
  }

  /**
   * @brief Image copies the data from other to the created object
   * @param other the copy source
   * @author Arne Hasselbring
   */
  Image(const Image& other)
    : size_(other.size_)
    , data_(new Color[size_.x() * size_.y()])
    , name_(other.name_)
    , real_size_(size_)
  {
    std::memcpy(data_, other.data_, size_.x() * size_.y() * sizeof(Color));
  }

  /**
   * @brief ~Image frees the data of the image
   * @author Arne Hasselbring
   */
  ~Image()
  {
    delete[] data_;
  }

  /**
   * @brief operator= copies an image
   * @param other the copy source
   * @return the copied object
   * @author Arne Hasselbring
   */
  Image operator=(const Image& other)
  {
    if (this == &other)
    {
      return *this;
    }
    if ((real_size_.x() * real_size_.y()) < (other.size_.x() * other.size_.y()))
    {
      if (data_)
      {
        delete[] data_;
      }
      data_ = new Color[other.size_.x() * other.size_.y()];
      real_size_ = other.size_;
    }
    name_ = other.name_;
    size_ = other.size_;
    std::memcpy(data_, other.data_, size_.x() * size_.y() * sizeof(Color));
    return *this;
  }

  /**
   * @brief resize sets the size of an image and preallocates memory
   * @param size the new size of the image
   * @author Arne Hasselbring
   */
  void resize(const Vector2i& size)
  {
    if ((real_size_.x() * real_size_.y()) < (size.x() * size.y()))
    {
      if (data_)
      {
        delete[] data_;
      }
      data_ = new Color[size.x() * size.y()];
      real_size_ = size;
    }
    size_ = size;
  }

  /**
   * @brief operator[] returns a reference to the color identified by a vector
   * @param coords a vector that identifies the desired position
   * @return a reference to the pixel data in the image
   * @author Arne Hasselbring
   */
  Color& operator[](const Vector2i& coords)
  {
#ifdef IMAGE_DEBUG
    if (!isInside(coords))
    {
      throw std::runtime_error("Tried to access image out of bounds with operator[]!");
    }
#endif
    return data_[coords.y() * size_.x() + coords.x()];
  }

  /**
   * @brief operator[] returns a constant reference to the color identified by a vector
   * @param coords a vector that identifies the desired position
   * @return a constant reference to the pixel data in the image
   * @author Arne Hasselbring
   */
  const Color& operator[](const Vector2i& coords) const
  {
#ifdef IMAGE_DEBUG
    if (!isInside(coords))
    {
      throw std::runtime_error("Tried to access image out of bounds with operator[]!");
    }
#endif
    return data_[coords.y() * size_.x() + coords.x()];
  }

  /**
   * @brief at returns a reference to the color identified by two coordinates
   * @param y the y coordinate (down) of the pixel
   * @param x the x coordinate (right) of the pixel
   * @return a reference to the pixel data in the image
   * @author Arne Hasselbring
   */
  Color& at(unsigned int y, unsigned int x)
  {
#ifdef IMAGE_DEBUG
    if (!isInside(y, x))
    {
      throw std::runtime_error("Tried to access image out of bounds with at!");
    }
#endif
    return data_[y * size_.x() + x];
  }

  /**
   * @brief at returns a reference to the color identified by two coordinates
   * @param point the x, y coordinate of the pixel
   * @return a reference to the pixel data in the image
   * @author Nicolas Riebesel
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
   * @author Arne Hasselbring
   */
  const Color& at(unsigned int y, unsigned int x) const
  {
#ifdef IMAGE_DEBUG
    if (!isInside(y, x))
    {
      throw std::runtime_error("Tried to access image out of bounds with at!");
    }
#endif
    return data_[y * size_.x() + x];
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
   * @author Arne Hasselbring
   */
  bool isInside(const Vector2i& coords) const
  {
    return ((coords.x() >= 0) && (coords.y() >= 0) && (coords.x() < size_.x()) &&
            (coords.y() < size_.y()));
  }

  /**
   * @brief isInside checks if a given point is inside the image
   * @param y the y coordinate (down) of the pixel
   * @param x the x coordinate (right) of the pixel
   * @return true if the point is inside the image
   * @author Arne Hasselbring
   */
  bool isInside(int y, int x) const
  {
    return ((x >= 0) && (y >= 0) && (x < size_.x()) && (y < size_.y()));
  }

  /**
   * @brief line draws a line between to points
   * @param p1 start point
   * @param p2 end point
   * @param color the color of the line
   * @return false if the line lies outside of the image
   * @author Arne Hasselbring, Thomas Schattschneider
   */
  bool line(const Vector2i& p1, const Vector2i& p2, const Color& color);

  /**
   * @brief draws a line on the image
   *
   * This simply wraps the line function that takes two points as arguments.
   *
   * @param l line to draw
   * @param color the color of the line
   * @return false if the line lies outside of the image
   * @author Thomas Schattschneider
   */
  bool line(const Line<int>& l, const Color& color);

  /**
   * @brief rectangle draws a rectangle around a center point
   * @param center the center of the rectangle
   * @param width the width of the rectangle
   * @param height the height of the rectangle
   * @param color the color of the rectangle
   * @author Chris Kahlefendt
   */
  void rectangle(const Vector2i& center, int width, int height, const Color& color);

  /**
   * @brief rectangle draws a rectangle by passing two points
   * @param p1 point that is going to be the upper left corner of the rectangle
   * @param p2 point that is going to be the lower right corner of the rectangle
   * @param color the color of the rectangle
   * @author Chris Kahlefendt
   */
  void rectangle(const Vector2i& p1, const Vector2i& p2, const Color& color);

  /**
   * @brief rectangle draws a rectangle by passing a rectangle
   * @param r the rectangle to draw
   * @param color the color of the rectangle
   * @author Georg Felbinger
   */
  void rectangle(const Rectangle<int>& r, const Color& color);

  /**
   * @brief circle draws a circle around a point
   * @param center the center point of the circle
   * @param radius the radius of the circle
   * @param color the color of the circle
   * @author Arne Hasselbring
   */
  void circle(const Vector2i& center, int radius, const Color& color);
  /**
   * @brief ellipse draws a ellipse around a point with given axes
   * @param center the center point of the ellipse in pixel coordinates
   * @param axes the length of the ellipse axes in pixels ("~radius")
   * @param rotation the rotation of the ellipse in radians
   * @param color the color the ellipse is going to be drawn in
   * @param resolution increasing this improves the quality of the drawn shape
   *                   (high values will result in high computational cost)
   */
  void ellipse(const Vector2i& center, const Vector2i& axes, const float rotation,
               const Color& color, const int resolution = 10);

  /**
   * @brief draws a colored cross to mark a point
   * @param center the center point of the cross
   * @param color the color of the cross
   * @return false if the cross lies outside of the image
   * @author Thomas Schattschneider
   */
  bool cross(const Vector2i& center, const int& size, const Color& color);

  /**
   * @brief draws a histogram on top of the image
   * @param values the histogram values. The number of values correlates to the number of boxes
   * @param color the color of the boxes in the histogram
   * @param precision the floating point precision to display (0 for no values to draw)
   * @param maxValue [optional] Whether to use the max from the given values or your own
   */
  void histogram(const std::vector<int>& values, const Color& color, unsigned int precision,
                 float maxValue = 0.f);

  /**
   * @brief draws a histogram on top of the image
   * @param values the histogram values. The number of values correlates to the number of boxes
   * @param color the color of the boxes in the histogram
   * @param precision the floating point precision to display (0 for no values to draw)
   * @param maxValue [optional] Whether to use the max from the given values or your own
   */

  void histogram(const std::vector<float>& values, const Color& color, unsigned int precision,
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
   * @param image the image of ascii symbols
   * @param str the string to draw
   * @param position the position where to draw
   * @param color font color
   * @return whether the writing was successful or not
   */
  bool drawString(const std::string& str, const Vector2i& position, const Color& color);

  /**
   * @brief drawPolygon draws polygon edges
   * @param polygon the polygon to draw
   * @param color the polygon color
   * @return whether the drawing of all edges was successful
   */
  bool drawPolygon(const Polygon<int>& polygon, const Color& color);

  /// the dimensions of the image
  Vector2i size_;
  /// the image data, saved row by row
  Color* data_;
  /// the image name, e.g. full path in replay
  std::string name_;

private:
  /**
   * @brief clipLine clips a line to the image frame
   * @param pt1 start point of a line
   * @param pt2 end point of a line
   * @return false if the line lies oustide of the image
   * @author Thomas Schattschneider
   */
  bool clipLine(Vector2i& pt1, Vector2i& pt2);
  /// the size for which memory is allocated
  Vector2i real_size_;
};
