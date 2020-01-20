#pragma once

#include "Eigen.hpp"

#include <Tools/Storage/UniValue/UniValue.h>

template <typename T>
class Plane : public Uni::From, public Uni::To
{
public:
  /**
   * @brief Plane constructor
   * @param origin the plane origin
   * @param normal the plane normal
   */
  Plane(const Vector2<T>& origin, const Vector2<T> normal)
    : origin(origin)
    , normal(normal)
  {
  }

  /**
   * @brief fromValue converts a Uni::Value to this
   * @param value the value that should be converted to this class
   */
  void fromValue(const Uni::Value& value)
  {
    assert(value.type() == Uni::ValueType::ARRAY);
    assert(value.size() == 1);
    value.at(0) >> origin;
    value.at(1) >> normal;
  }
  /**
   * @brief toValue converts this to a Uni::Value
   * @param value the value that this class should be converted to
   */
  void toValue(Uni::Value& value) const
  {
    value = Uni::Value(Uni::ValueType::ARRAY);
    value.at(0) << origin;
    value.at(1) << normal;
  }

  /// the plane origin
  Vector2<T> origin;
  /// the plane normal
  Vector2<T> normal;
};
