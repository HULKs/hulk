#pragma once
#include <algorithm>

#include "Angle.hpp"
#include "Eigen.hpp"
#include "Pose.hpp"
#include "Tools/Storage/UniValue/UniValue.h"

class Velocity : public Uni::To, public Uni::From
{
public:
  /**
   * @brief Construct a velocity with translational and rotational component. Defaults to maximum
   * velocities.
   *
   * @param translation Movement speed in a specific direction. This can be specified in percentage
   * mode (default) or m/s. When using the percentage mode, the length of the vector will be
   * interpreted as the desired percentage (0 - 1) of the maximum velocity. Explicitly set this
   * parameter when in velocity walking mode to specify direction and velocity.
   * @param rotation Rotation speed, given as a percentage (default, 0 - 1) or in rad/s.
   * @param usePercentage If true (default), all values will be interpreted as percentages of
   * maximum velocites.
   */
  Velocity(const Vector2f& translation = Vector2f(1.f, 0.f), float rotation = 1.f,
           bool usePercentage = true)
    : translation(translation)
    , rotation(rotation)
    , usePercentage_(usePercentage)
  {
    // Percentage values should always be in the interval of [0, 1]
    assert(!usePercentage || (translation.norm() <= 1 && std::abs(rotation) <= 1));
  }
  /**
   * @brief Construct a velocity with translational and rotational magnitude
   *
   * @param translation Movement speed, given as a percentage (default, 0 - 1) or in m/s.
   * @param rotation Rotation speed as a percentage (default, 0 - 1) or in rad/s.
   * @param usePercentage If true (default), all values will be interpreted as percentages of
   * maximum velocites.
   *
   * @note Usually this constructor should be used to specify a velocity.
   */
  Velocity(float translation, float rotation = 1.f, bool usePercentage = true)
    : translation(Vector2f(translation, 0))
    , rotation(rotation)
    , usePercentage_(usePercentage)
  {
    // Percentage values should always be in the interval of [0, 1]
    assert(!usePercentage ||
           (translation >= 0 && translation <= 1 && rotation >= 0 && rotation <= 1));
  }
  /**
   * @brief Return true if percentage values were specified
   * @return True if percentage values were specified
   */
  bool isPercentage() const
  {
    return usePercentage_;
  }
  /**
   * @brief getAbsolute Calculates a Velocity object representing the corresponding absolute
   * velocity from given maximum velocity components. Note: If the velocity is already absolute
   * (isPercentage() returns false) the the object will be simply copied, not clipping it to the
   * velocityComponents!
   * @param maxVelocityComponents The maximum velocities in all three directions (x, y, alpha). These
   * are the maximum velocities that would be possible in each of the directions alone! The
   * maxVelocityComponents object does NOT encode any direction information!
   */
  Velocity getAbsolute(const Pose& maxVelocityComponents) const
  {
    // make sure the user understood how to use maxVelocityComponents
    assert(maxVelocityComponents.position.x() >= 0.f && maxVelocityComponents.position.y() >= 0.f &&
           maxVelocityComponents.orientation >= 0.f);
    // if the velocity is already absolute, it remains unchanged.
    if (!usePercentage_)
    {
      return *this;
    }
    // extract the maximum velocities from the maxVelocityComponents
    const float maxTranslationVelocity =
        std::max(maxVelocityComponents.position.x(), maxVelocityComponents.position.y());
    // transform to aboslute velocity while not changing direction
    return {translation.normalized() * maxTranslationVelocity,
            rotation * maxVelocityComponents.orientation, false};
  }
  /**
   * @brief toValue converts a velocity into a Uni::Value
   * @param value where to store the converted value
   */
  void toValue(Uni::Value& value) const
  {
    value = Uni::Value(Uni::ValueType::ARRAY);
    value.reserve(3);
    value.at(0) << translation;
    value.at(1) << rotation;
    value.at(2) << usePercentage_;
  }
  /**
   * @brief fromValue converts a Uni::Value into a velocity
   * @param value a Uni::Value, e.g. from config
   */
  void fromValue(const Uni::Value& value)
  {
    assert(value.type() == Uni::ValueType::ARRAY);
    assert(value.size() == 3);
    value.at(0) >> translation;
    value.at(1) >> rotation;
    value.at(2) >> usePercentage_;
  }
  /**
   * Movement velocity given as a percentage (default, 0 - 1) or in m/s.
   * When in path mode, only the length of the vector matters. Use the appropriate constructor.
   */
  Vector2f translation;
  /// Rotation speed as a percentage (preferred, 0 - 1) or in rad/s.
  float rotation;

private:
  /// True if velocities are given as percentages of maximum velocities
  bool usePercentage_;
};
