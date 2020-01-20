#pragma once

#include "Framework/DataType.hpp"

struct IntegralImage
{
  IntegralImage()
    : size(Vector2i::Zero())
  {
  }
  IntegralImage(const Vector2i& size, const int& scale)
    : size(size)
    , scale(scale)
    , data(nullptr)
  {
    if (size != Vector2i::Zero())
    {
      data = new unsigned int[size.y() * size.x()];
    }
  }

  ~IntegralImage()
  {
    delete[] data;
  }

  void resize(const Vector2i& newSize)
  {
    if (newSize != size)
    {
      delete[] data;
      size = newSize;
      if (size != Vector2i::Zero())
      {
        data = new unsigned int[size.y() * size.x()];
      }
      else
      {
        data = nullptr;
      }
    }
  }

  unsigned int& at(size_t y, size_t x) const
  {
    return data[x + y * size.x()];
  }

  Vector2i size;
  int scale{0};
  unsigned int* data{nullptr};
};

class IntegralImageData : public DataType<IntegralImageData>
{
public:
  /// the name of this DataType
  DataTypeName name = "IntegralImageData";

  IntegralImage image;
  bool valid = false;

  /*
   * @brief gets the integral value of a given rectangle by upperLeft and lowerRight corners
   * Because each pixel in an integral image represents the sum off all previous pixel values up to
   * this pixel the sum of an rectangle can be computed by adding the top left and bottom right
   * corner pixel values and substracting the area counted twice by subtracting the values of the
   * top right and bottom left pixel values.
   * @param upperLeft the upper left position of the rectangle
   * @param lowerRight the lower right position of the rectangle
   * @return the sum of all pixel values in the given rectangle
   */
  unsigned int getIntegralValue(const Vector2i& upperLeft, const Vector2i& lowerRight) const
  {
    assert(lowerRight.x() >= upperLeft.x());
    assert(lowerRight.y() >= upperLeft.y());
    assert(image.at(lowerRight.y(), lowerRight.x()) >= image.at(upperLeft.y(), upperLeft.x()));
    return (image.at(lowerRight.y(), lowerRight.x()) + image.at(upperLeft.y(), upperLeft.x()) -
            image.at(upperLeft.y(), lowerRight.x()) - image.at(lowerRight.y(), upperLeft.x()));
  }

  void reset() override
  {
    valid = false;
  }

  void toValue(Uni::Value& value) const override
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["valid"] << valid;
  }

  void fromValue(const Uni::Value& value) override
  {
    value["valid"] >> valid;
  }
};
