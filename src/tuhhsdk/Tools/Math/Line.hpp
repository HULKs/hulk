#pragma once

#include "Eigen.hpp"

#include <Tools/Storage/UniValue/UniConvertible.hpp>

template <typename T>
class Line : public Uni::From, public Uni::To
{
public:
  /**
   * @brief Line constructs a line from (0, 0) to (0, 0)
   * @param p1 start point of the line
   * @param p2 end point of the line
   */
  Line(const Vector2<T>& p1 = Vector2<T>::Zero(), const Vector2<T>& p2 = Vector2<T>::Zero())
    : p1(p1)
    , p2(p2)
  {
  }

  /**
   * @brief getY can calculate the y coordinate of a given x coordinate
   * @param x the x coordinate
   * @return the calculated y coordinate
   */
  T getY(T x) const
  {
    assert(p1.x() != p2.x());
    return static_cast<float>(p2.y() - p1.y()) / static_cast<float>(p2.x() - p1.x()) *
               (x - p1.x()) +
           p1.y();
  }

  /**
   * @brief getX can calculate the x coordinate of a given y coordinate
   * @param y the y coordinate
   * @return the calculated x coordinate
   */
  T getX(T y) const
  {
    assert(p1.y() != p2.y());
    return static_cast<float>(p2.x() - p1.x()) / static_cast<float>(p2.y() - p1.y()) *
               (y - p1.y()) +
           p1.x();
  }
  /**
   * @brief fromValue converts a Uni::Value to this
   * @param value the value that should be converted to this class
   */
  void fromValue(const Uni::Value& value)
  {
    assert(value.type() == Uni::ValueType::ARRAY);
    assert(value.size() == 2);
    value.at(0) >> p1;
    value.at(1) >> p2;
  }
  /**
   * @brief toValue converts this to a Uni::Value
   * @param value the value that this class should be converted to
   */
  void toValue(Uni::Value& value) const
  {
    value = Uni::Value(Uni::ValueType::ARRAY);
    value.at(0) << p1;
    value.at(1) << p2;
  }

  /// start point of the line
  Vector2<T> p1;
  /// end point of the line
  Vector2<T> p2;
};

