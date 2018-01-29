#pragma once

#include <Tools/Math/Eigen.hpp>
#include <Tools/Storage/UniValue/UniConvertible.hpp>

/**
 * @brief The circle struct
 * @autor Erik Schröder
 */
template <typename T>
class Circle : public Uni::From, public Uni::To
{
public:
  /**
   * @brief Circle constructs a circle with radius 0
   * @param center the center of the circle
   * @param radius the radius of the circle
   */
  Circle(const Vector2<T>& center = Vector2<T>::Zero(), T radius = 0)
    : center(center)
    , radius(radius)
  {
  }
  /**
   * @brief fromValue converts a Uni::Value to this
   * @param value the value that should be converted to this class
   */
  void fromValue(const Uni::Value& value)
  {
    assert(value.type() == Uni::ValueType::ARRAY);
    assert(value.size() == 2);
    value.at(0) >> center;
    value.at(1) >> radius;
  }
  /**
   * @brief toValue converts this to a Uni::Value
   * @param value the value that this class should be converted to
   */
  void toValue(Uni::Value& value) const
  {
    value = Uni::Value(Uni::ValueType::ARRAY);
    value.at(0) << center;
    value.at(1) << radius;
  }
  /// the center of the circle
  Vector2<T> center;
  /// the radius of the circle
  T radius;
};
