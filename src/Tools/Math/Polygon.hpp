#pragma once

#include "Tools/Storage/UniValue/UniValue.h"
#include <algorithm>

template <typename T>
struct Polygon : public Uni::From, public Uni::To
{
  Polygon() = default;

  /**
   * @brief Polygon constructor
   * @param points the points forming the polygon
   */
  explicit Polygon(std::vector<Vector2<T>> points);

  // Points need to be in counter-/clockwise order
  float area() const;

  /**
   * @brief Get the Polygon as YUV444 from YUV422 coordinates. Method adds points to
   * given Polygon.
   */
  Polygon<T> get444from422() const;

  /**
   * @brief Get the Polygon as YUV422 from YUV444 coordinates. Method adds points to
   * given Polygon.
   */
  Polygon<T> get422from444() const;

  /**
   * @brief Converts the Polygon from YUV422 to YUV444 coordinates. Method adds points to
   * given Polygon.
   */
  void convertFrom422to444();

  /**
   * @brief Converts the Polygon from YUV444 to YUV422 coordinates. Method adds points to
   * given Polygon.
   */
  void convertFrom444to422();

  /**
   * @brief fromValue converts a Uni::Value to this
   * @param value the value that should be converted to this class
   */
  void fromValue(const Uni::Value& value) override;

  /**
   * @brief toValue converts this to a Uni::Value
   * @param value the value that this class should be converted to
   */
  void toValue(Uni::Value& value) const override;

  /// the points of the polygon in mathematically positive order
  std::vector<Vector2<T>> points;
};

template <typename T>
Polygon<T>::Polygon(std::vector<Vector2<T>> points)
  : points{std::move(points)}
{
}

template <typename T>
float Polygon<T>::area() const
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

template <typename T>
Polygon<T> Polygon<T>::get444from422() const
{
  Polygon<T> converted444;
  converted444.points.reserve(points.size());
  std::transform(points.begin(), points.end(), std::back_inserter(converted444.points),
                 [](const auto& point) {
                   return Vector2<T>{point.x() * T{2}, point.y()};
                 });
  return converted444;
}

template <typename T>
Polygon<T> Polygon<T>::get422from444() const
{
  Polygon<T> converted422;
  converted422.points.reserve(points.size());
  std::transform(points.begin(), points.end(), std::back_inserter(converted422.points),
                 [](const auto& point) {
                   return Vector2<T>{point.x() / T{2}, point.y()};
                 });
  return converted422;
}

template <typename T>
void Polygon<T>::convertFrom422to444()
{
  std::transform(points.begin(), points.end(), points.begin(), [](const auto& point) {
    return Vector2<T>{point.x() * T{2}, point.y()};
  });
}

template <typename T>
void Polygon<T>::convertFrom444to422()
{
  std::transform(points.begin(), points.end(), points.begin(), [](const auto& point) {
    return Vector2<T>{point.x() / T{2}, point.y()};
  });
}

template <typename T>
void Polygon<T>::fromValue(const Uni::Value& value)
{
  assert(value.type() == Uni::ValueType::ARRAY);
  value >> points;
}

template <typename T>
void Polygon<T>::toValue(Uni::Value& value) const
{
  value = Uni::Value{Uni::ValueType::ARRAY};
  value << points;
}
