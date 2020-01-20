#pragma once

#include <Tools/Storage/UniValue/UniValue.h>

/**
 * @brief The circular arc class - short arc.
 * @autor elixF armuthW
 */
template <typename T>
class Arc : public Uni::From, public Uni::To
{
public:
  /**
   * @brief Arc constructs a circular arc from angle 0 to 0 and a radius of -1
   * @param center of the circle
   * @param radius of the circle
   * @param startAngle of the arc
   * @param endAngle of the arc
   * @param clockwise - direction of arc, cw or ccw.
   */
  Arc(const Vector2<T>& center = Vector2<T>::Zero(), T radius = -1, T startAngle = 0,
      T endAngle = 0, bool clockwise = false)
    : center(center)
    , radius(radius)
    , startAngle(startAngle)
    , endAngle(endAngle)
    , clockwise(clockwise)
  {
  }

  /**
   * @brief fromValue converts a Uni::Value to this
   * @param value the value that should be converted to this class
   */
  void fromValue(const Uni::Value& value)
  {
    assert(value.type() == Uni::ValueType::OBJECT);
    assert(value.size() == 5);
    value["center"] >> center;
    value["radius"] >> radius;
    value["startAngle"] >> startAngle;
    value["endAngle"] >> endAngle;
    value["clockwise"] >> clockwise;
  }

  /**
   * @brief toValue converts this to a Uni::Value
   * @param value the value that this class should be converted to
   */
  void toValue(Uni::Value& value) const
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["center"] << center;
    value["radius"] << radius;
    value["startAngle"] << startAngle;
    value["endAngle"] << endAngle;
    value["clockwise"] << clockwise;
  }

  /// the center of the circle
  Vector2<T> center;
  /// the radius of the circle
  T radius;
  /// angle at which the arc starts
  T startAngle;
  /// angle at which the arc ends
  T endAngle;
  /// direction of the arc
  bool clockwise;
};
