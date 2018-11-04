#pragma once

#include <Tools/Storage/UniValue/UniConvertible.hpp>

template <typename T>
class Polygon : public Uni::From, public Uni::To
{
public:
  Polygon() = default;

  /**
   * @brief Polygon constructor
   * @param points the points forming the polygon
   */
  Polygon(const std::vector<Vector2<T>>& points)
    : points(points)
  {
  }

  // Points need to be in counter-/clockwise order
  float area() const
  {
    float area = 0;
    int n = this->points.size();
    // Calculate value of shoelace formula
    int j = n - 1;
    for (int i = 0; i < n; i++)
    {
      area +=
          (this->points[j].x() + this->points[i].x()) * (this->points[j].y() - this->points[i].y());
      j = i; // j is previous vertex to i
    }

    // Return absolute value
    return std::abs(area / 2);
  }

  /**
   * @brief fromValue converts a Uni::Value to this
   * @param value the value that should be converted to this class
   */
  void fromValue(const Uni::Value& value)
  {
    assert(value.type() == Uni::ValueType::ARRAY);
    assert(value.size() == 1);
    value.at(0) >> points;
  }
  /**
   * @brief toValue converts this to a Uni::Value
   * @param value the value that this class should be converted to
   */
  void toValue(Uni::Value& value) const
  {
    value = Uni::Value(Uni::ValueType::ARRAY);
    value.at(0) << points;
  }

  /// the points of the polygon in mathematically positive order
  std::vector<Vector2<T>> points;
};
