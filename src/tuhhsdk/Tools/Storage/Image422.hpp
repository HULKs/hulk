#pragma once

#include <cstdint>
#include <cstring>
#include <stdexcept>

#include "Tools/Math/Eigen.hpp"
#include "Tools/Math/Line.hpp"
#include "Tools/Storage/Image.hpp"

// Uncomment this for range checks. This will slow down every image access.
// #define IMAGE_DEBUG

struct YCbCr422
{
  /**
   * @brief YCbCr422 initializes the channels with 0
   */
  YCbCr422()
    : y1_(0)
    , cb_(0)
    , y2_(0)
    , cr_(0)
  {
  }
  /**
   * @brief YCbCr422 initializes the channels with user-defines values
   * @param y1 the initial value for the y channel
   * @param cb the initial value for the cb channel
   * @param y2 the initial value for the y channel
   * @param cr the initial value for the cr channel
   */
  YCbCr422(std::uint8_t y1, std::uint8_t cb, std::uint8_t y2, std::uint8_t cr)
    : y1_(y1)
    , cb_(cb)
    , y2_(y2)
    , cr_(cr)
  {
  }
  /// y1 channel
  std::uint8_t y1_;
  /// cb/u channel
  std::uint8_t cb_;
  /// y2 channel
  std::uint8_t y2_;
  /// cr/v channel
  std::uint8_t cr_;

  /**
   * @brief Comparison with another color
   */
  bool operator==(const YCbCr422& other) const
  {
    return y1_ == other.y1_ && cb_ == other.cb_ && y2_ == other.y2_ && cr_ == other.cr_;
  }

  /**
   * @brief Calculates average over luminance
   */
  std::uint8_t averagedY() const
  {
    return ((uint16_t)y1_ + y2_) >> 1;
  }
};

class Image422
{
public:
  /**
   * @brief Image422 initializes an empty image
   */
  Image422()
    : size(0, 0)
    , data(nullptr)
    , realSize_(0, 0)
    , isExternalData_(false)
  {
  }

  /**
   * @brief Image422 allocates memory for an image of the specified size
   * @param size the dimensions of the new image
   */
  explicit Image422(const Vector2i& size)
    : size(get422From444Vector(size))
    , data(new YCbCr422[calculateNeededSpace(size)])
    , realSize_(size)
    , isExternalData_(false)
  {
  }

  /**
   * @brief Image422 allocates memory for an image of the specified size and sets color
   * @param size the dimensions of the new image
   * @param color of created image
   */
  Image422(const Vector2i& size, const YCbCr422& color)
    : size(get422From444Vector(size))
    , data(new YCbCr422[calculateNeededSpace(size)])
    , realSize_(size)
    , isExternalData_(false)
  {
    for (unsigned int i = 0; i < calculateNeededSpace(size); i++)
    {
      data[i] = color;
    }
  }

  /**
   * @brief Image422 wraps YUV422 from another memory location into an Image422 object
   * @param size Size of the image data
   * @param data Pointer to the image data
   */
  Image422(const Vector2i& size, YCbCr422* data)
    : size(get422From444Vector(size))
    , data(data)
    , realSize_(size)
    , isExternalData_(true)
  {
  }

  /**
   * @brief Image422 copies the data from other to the created object
   * @param other the copy source
   */
  Image422(const Image422& other)
    : size(other.size)
    , data(new YCbCr422[calculateNeededSpace(size)])
    , realSize_(size)
    , isExternalData_(false)
  {
    std::memcpy(data, other.data, calculateNeededSpace(size) * sizeof(YCbCr422));
  }

  /**
   * @brief ~Image422 frees the data of the image
   */
  ~Image422()
  {
    if (!isExternalData_ && data)
    {
      delete[] data;
    }
  }

  /**
   * @brief operator= copies an image
   * @param other the copy source
   * @return the copied object
   */
  Image422& operator=(const Image422& other)
  {
    if (this == &other)
    {
      return *this;
    }
    if (calculateNeededSpace(realSize_) < calculateNeededSpace(other.size))
    {
      if (data)
      {
        delete[] data;
      }
      data = new YCbCr422[calculateNeededSpace(other.size)];
      realSize_ = other.size;
    }
    size = other.size;
    isExternalData_ = false;
    std::memcpy(data, other.data, calculateNeededSpace(other.size) * sizeof(YCbCr422));
    return *this;
  }

  /**
   * @brief resize sets the size of an 444 image and preallocates memory
   * @param size the new size of the image (444)
   */
  void resize(const Vector2i& size)
  {
    auto sizeFor422 = get422From444Vector(size);
    if (calculateNeededSpace(realSize_) < calculateNeededSpace(sizeFor422))
    {
      if (data)
      {
        delete[] data;
      }
      data = new YCbCr422[calculateNeededSpace(sizeFor422)];
      realSize_ = sizeFor422;
    }
    this->size = sizeFor422;
  }

  /**
   * @brief setData lets you set the data to externally managed memory
   * @param data Pointer to the external memory region
   * @param size The 444 size of the data in memory
   */
  void setData(YCbCr422* data, const Vector2i& size)
  {
    if (!isExternalData_ && this->data)
    {
      delete[] this->data;
    }

    this->data = data;
    this->size = get422From444Vector(size);
    realSize_ = this->size;
    isExternalData_ = true;
  }

  /**
   * @brief operator[] returns a reference to the color identified by a vector
   * @param coords a vector that identifies the desired position
   * @return a reference to the pixel data in the image
   */
  YCbCr422& operator[](const Vector2i& coords)
  {
#ifdef IMAGE_DEBUG
    if (!isInside(coords))
    {
      throw std::runtime_error("Tried to access image out of bounds with operator[]!");
    }
#endif
    return data[calculateCoordPositionInArray(coords)];
  }

  /**
   * @brief operator[] returns a constant reference to the color identified by a vector
   * @param coords a vector that identifies the desired position
   * @return a constant reference to the pixel data in the image
   */
  const YCbCr422& operator[](const Vector2i& coords) const
  {
#ifdef IMAGE_DEBUG
    if (!isInside(coords))
    {
      throw std::runtime_error("Tried to access image out of bounds with operator[]!");
    }
#endif
    return data[calculateCoordPositionInArray(coords)];
  }

  /**
   * @brief at returns a reference to the color identified by two coordinates
   * @param y the y coordinate (down) of the pixel
   * @param x the x coordinate (right) of the pixel
   * @return a reference to the pixel data in the image
   */
  YCbCr422& at(unsigned int y, unsigned int x)
  {
#ifdef IMAGE_DEBUG
    if (!isInside(y, x))
    {
      throw std::runtime_error("Tried to access image out of bounds with at!");
    }
#endif
    return data[calculateCoordPositionInArray(y, x)];
  }

  /**
   * @brief at returns a constant reference to the color identified by two coordinates
   * @param y the y coordinate (down) of the pixel
   * @param x the x coordinate (right) of the pixel
   * @return a constant reference to the pixel data in the image
   */
  const YCbCr422& at(unsigned int y, unsigned int x) const
  {
#ifdef IMAGE_DEBUG
    if (!isInside(y, x))
    {
      throw std::runtime_error("Tried to access image out of bounds with at!");
    }
#endif
    return data[calculateCoordPositionInArray(y, x)];
  }

  /**
   * @brief at returns a constant reference to the color identified by two coordinates
   * @param point the x, y coordinate of the pixel
   * @return a constant reference to the pixel data in the image
   */
  const YCbCr422& at(const Vector2i& point) const
  {
    return at(point.y(), point.x());
  }

  /**
   * @brief to444Image convert yuv422 image to yuv444 image
   * @param image the image to copy the data in
   */
  void to444Image(Image& image) const;

  /**
   * @brief to444Image convert yuv422 image to yuv444 image
   * @return return the 444 image
   */
  Image to444Image() const;


  /**
   * @brief isInside checks if a given point is inside the image
   * @param y the y coordinate (down) of the pixel
   * @param x the x coordinate (right) of the pixel
   * @return true if the point is inside the image
   */
  inline bool isInside(int y, int x) const
  {
    return ((x >= 0) && (y >= 0) && (x < size.x()) && (y < size.y()));
  }

  /**
   * @brief isInside checks if a given point is inside the image
   * @param coords a vector that identifies the point
   * @return true if the point is inside the image
   */
  inline bool isInside(const Vector2i& coords) const
  {
    return isInside(coords.y(), coords.x());
  }

  /**
   * @brief calculateNeededSpace calculates the number of YCbCr422 instances needed to be allocated
   * for a given 422 image size
   * @param size the width and height of the 422 image
   */
  inline std::size_t calculateNeededSpace(const Vector2i& size) const
  {
    return size.x() * size.y();
  }

  /**
   * @brief calculateCoordPosisionInArray calculates the position of the specified coordinates
   * inside the array
   * @param y the y coordinate of pixel in question
   * @param x the x coordinate of pixel in question
   */
  inline std::size_t calculateCoordPositionInArray(unsigned int y, unsigned int x) const
  {
    return y * size.x() + x;
  }

  /**
   * @brief calculateCoordPosisionInArray calculates the position of the specified coordinates
   * inside the array
   * @param coord the coordinate of pixel in question
   */
  inline std::size_t calculateCoordPositionInArray(const Vector2i& coord) const
  {
    return calculateCoordPositionInArray(coord.y(), coord.x());
  }

  /**
   * @brief get422From444Vector calculates the 422 image coordinate vector from a 444 image
   * coordinate vector
   * @param v444 the 444 image coordinate
   */
  static inline Vector2i get422From444Vector(const Vector2i& v444)
  {
    return {v444.x() / 2, v444.y()};
  }

  /**
   * @brief get422From444Vector calculates the 422 image coordinate vector from a 444 coordinaes
   * @param x the x coordinate in 444
   * @param y the y coordinate in 444
   */
  static inline Vector2i get422From444Vector(const int x, const int y)
  {
    return {x / 2, y};
  }

  /**
   * @brief get444From422Vector calculates the 444 image coordinate vector from a 422 image
   * coordinate vector
   * @param v422 the 422 image coordinate
   */
  static inline Vector2i get444From422Vector(const Vector2i& v422)
  {
    return {v422.x() * 2, v422.y()};
  }

  /**
   * @brief get422From444Vector calculates the 422 image coordinate vector from a 444 coordinaes
   * @param x the x coordinate in 422
   * @param y the y coordinate in 422
   */
  static inline Vector2i get444From422Vector(const int x, const int y)
  {
    return {x * 2, y};
  }

public:
  /// the dimensions of the image (422)
  Vector2i size;
  /// the image data, saved row by row
  YCbCr422* data;

private:
  /// the size for which memory is allocated
  Vector2i realSize_;
  /// true if the data pointed to is external
  bool isExternalData_;

  /// for SSE 422 to 444 conversion
  static char shuffle1[16];
  static char shuffle2[16];
  static char shuffle3[16];
};
