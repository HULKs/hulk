#pragma once

#include "Tools/Math/Circle.hpp"
#include "Tools/Storage/UniValue/UniValue.h"

/**
 * @brief The circular arc class - short arc.
 * @author elixF armuthW
 */
template <typename T>
class Arc : public Uni::From, public Uni::To
{
public:
  Arc() = default;

  /**
   * @brief Arc constructs a circular arc between two points
   * @param circle the circle the arc is part of
   * @param start the start point of the arc on the circle
   * @param end the end point of the arc on the circle
   * @param clockwise whether the direction of the arc is clockwise
   */
  explicit Arc(const Circle<T>& circle, const Vector2<T>& start = Vector2<T>::Zero(),
               const Vector2<T>& end = Vector2<T>::Zero(), bool clockwise = false)
    : circle(circle)
    , start(start)
    , end(end)
    , relStart(start - circle.center)
    , relEnd(end - circle.center)
    , clockwise(clockwise)
  {
  }

  /**
   * @brief fromValue converts a Uni::Value to this
   * @param value the value that should be converted to this class
   */
  void fromValue(const Uni::Value& value) override
  {
    assert(value.type() == Uni::ValueType::OBJECT);
    assert(value.size() == 4);
    value["circle"] >> circle;
    value["start"] >> start;
    value["end"] >> end;
    value["clockwise"] >> clockwise;
  }

  /**
   * @brief toValue converts this to a Uni::Value
   * @param value the value that this class should be converted to
   */
  void toValue(Uni::Value& value) const override
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["circle"] << circle;
    value["start"] << start;
    value["end"] << end;
    value["clockwise"] << clockwise;
  }

  /// the circle the arc is part of
  Circle<T> circle;
  /// start point of arc
  Vector2<T> start;
  /// end point of arc
  Vector2<T> end;
  /// start point of arc relative to its center
  Vector2<T> relStart;
  /// end point of arc relative to its center
  Vector2<T> relEnd;
  /// direction of the arc
  bool clockwise{false};
};
