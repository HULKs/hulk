#pragma once

#include "Tools/Math/Polygon.hpp"

#include <Tools/Storage/UniValue/UniValue.h>

template <typename T>
class ConvexPolygon : public Polygon<T>
{
public:
  ConvexPolygon() = default;

  /**
   * @brief ConvexPolygon constructor
   * @param points the points forming a convex polygon
   */
  ConvexPolygon(const std::vector<Vector2<T>>& points)
    : Polygon<T>(points)
  {
  }

  /**
   * @brief fromValue converts a Uni::Value to this
   * @param value the value that should be converted to this class
   */
  void fromValue(const Uni::Value& /*value*/) {}
  /**
   * @brief toValue converts this to a Uni::Value
   * @param value the value that this class should be converted to
   */
  void toValue(Uni::Value& /*value*/) const {}
};
