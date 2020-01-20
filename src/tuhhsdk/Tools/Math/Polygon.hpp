#pragma once

#include <Tools/Storage/UniValue/UniValue.h>

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
   * @brief Converts a Polygon from YUV422 coordinates to YUV444 coordinates. Method adds points to
   * given Polygon.
   */
  void from422to444(Polygon<T>& polygon) const
  {
    for (const auto& point : points)
    {
      polygon.points.emplace_back(point.x() * 2, point.y());
    }
  }

  /**
   * @brief Converts a Polygon from YUV444 coordinates to YUV422 coordinates. Method adds points to
   * given Polygon.
   */
  void from444to422(Polygon<T>& polygon) const
  {
    for (const auto& point : points)
    {
      polygon.points.emplace_back(point.x() / 2, point.y());
    }
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
