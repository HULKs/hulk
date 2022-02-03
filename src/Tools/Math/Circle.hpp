#pragma once

#include "Tools/Math/Eigen.hpp"
#include "Tools/Storage/UniValue/UniValue.h"

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
   * @brief Circle copy constructor
   * @param other Circle object to copy the data from
   */

  Circle(const Circle<T>& other) = default;
  /**
   * @brief Circle move constructor
   * @param other Circle object to move the data from
   */
  Circle(Circle<T>&& other) noexcept = default;

  /**
   * @brief Circle copy assignment operator
   * @param Circle object to copy the data from
   */
  Circle& operator=(const Circle&) = default;

  /**
   * @brief Circle move assignment operator
   * @param Circle object to move the data from
   */
  Circle& operator=(Circle&&) noexcept = default;

  /**
   * @brief Circle destructor
   */
  virtual ~Circle() noexcept = default;

  /**
   * @brief Circle equality operator
   * @param other circle to compare to
   */
  bool operator==(const Circle& other) const
  {
    return center == other.center && radius == other.radius;
  }

  /**
   * @brief Get the Circle as YUV444 from YUV422 coordinates.
   */
  Circle<T> get444from422() const
  {
    return {{center.x() * 2, center.y()}, radius};
  }

  /**
   * @brief Get the Circle as YUV422 from YUV444 coordinates.
   */
  Circle<T> get422from444() const
  {
    return {{center.x() / 2, center.y()}, radius};
  }

  /**
   * @brief Converts this Circle from YUV422 to YUV444 coordinates.
   */
  void convertFrom422to444()
  {
    center.x() *= 2;
  }

  /**
   * @brief Converts this Circle from YUV422 to YUV444 coordinates.
   */
  void convertFrom444to422()
  {
    center.x() /= 2;
  }

  /**
   * @brief fromValue converts a Uni::Value to this
   * @param value the value that should be converted to this class
   */
  void fromValue(const Uni::Value& value) override
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
  void toValue(Uni::Value& value) const override
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
