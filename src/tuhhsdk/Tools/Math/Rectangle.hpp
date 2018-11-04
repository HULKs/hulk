#pragma once

#include <Tools/Math/Eigen.hpp>
#include <Tools/Storage/UniValue/UniConvertible.hpp>

/**
 * @brief Represents a rectangle by two Vector2<T>
 * @author Georg Felbinger
 */
template <typename T>
class Rectangle : public Uni::From, public Uni::To
{

public:
  /**
   * @brief Default constructor, ensures element wise topLeft <= bottomRight.
   */
  Rectangle(const Vector2<T>& a = Vector2<T>::Zero(), const Vector2<T>& b = Vector2<T>::Zero())
    : topLeft(a.cwiseMin(b))
    , bottomRight(a.cwiseMax(b))
  {
  }

  /**
   * @brief Copy constructor.
   */
  Rectangle(const Rectangle<T>& other)
    : topLeft(other.topLeft)
    , bottomRight(other.bottomRight)
  {
  }

  /// the top left point of the rectangle
  Vector2<T> topLeft;
  /// the bottom right point of the rectangle
  Vector2<T> bottomRight;

  /**
   * @brief  whether this rectangle overlaps with another one.
   * The overlap calculation exlcudes borders, e.g. ((2,2),(4,4)) and ((4,2),(6,4)) has no overlap.
   */
  bool hasOverlap(const Rectangle& other) const
  {
    const bool xOverlap =
        topLeft.x() < other.bottomRight.x() && bottomRight.x() > other.topLeft.x();
    if (!xOverlap)
    {
      return false;
    }
    const bool yOverlap = topLeft.y() < other.bottomRight.y() && bottomRight.y() > topLeft.y();
    if (!yOverlap)
    {
      return false;
    }
    return true;
  }

  /**
   * @brief calculates how much this rectangle overlaps with another one.
   * If there is no overlap or one of the rectangles has area==0, it will return 0.
   * Otherwise, it will calculate the ratio between the intersection area and the area of the
   * smaller rectangle.
   */
  float overlap(const Rectangle& other) const
  {
    const Vector2<T> intersectTopLeft = topLeft.cwiseMax(other.topLeft);
    const Vector2<T> intersectBottomRight = bottomRight.cwiseMin(other.bottomRight);
    const Vector2<T> intersectSize =
        (intersectBottomRight - intersectTopLeft).cwiseMax(Vector2<T>::Zero());
    const float intersectArea = intersectSize.x() * intersectSize.y();

    const float minArea = std::min(area(), other.area());
    if (minArea == 0)
    {
      return 0;
    }
    return intersectArea / minArea;
  }

  /**
   * @brief calculates the area of this rectangle.
   */
  float area() const
  {
    const Vector2<T> s = size();
    return static_cast<float>(s.x()) * s.y();
  }

  /**
   * @brief calculates the size vector of this rectangle.
   */
  Vector2<T> size() const
  {
    return bottomRight - topLeft;
  }

  /**
   * @brief Converts a Rectangle from YUV422 coordinates into YUV444 coordinates.
   */
  Rectangle<T> from422to444() const
  {
    Rectangle<T> converted(topLeft, bottomRight);
    converted.topLeft.x() *= 2;
    converted.bottomRight.x() *= 2;
    return converted;
  }

  void fromValue(const Uni::Value& value)
  {
    assert(value.type() == Uni::ValueType::ARRAY);
    assert(value.size() == 2);
    value.at(0) >> topLeft;
    value.at(1) >> bottomRight;
  }

  void toValue(Uni::Value& value) const
  {
    value = Uni::Value(Uni::ValueType::ARRAY);
    value.at(0) << topLeft;
    value.at(1) << bottomRight;
  }
};
