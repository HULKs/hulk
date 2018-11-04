#pragma once

#include <Tools/Storage/UniValue/UniConvertible.hpp>

#include "Tools/Math/Eigen.hpp"

class Pose : public Uni::To, public Uni::From
{
public:
  /**
   * @brief Pose constructs a new pose with given position and orientation
   * @param position the translational part of the pose
   * @param orientation an angle describing the orientation of the pose
   */
  Pose(const Vector2f& position = Vector2f::Zero(), const float orientation = 0)
    : position(position)
    , orientation(orientation)
  {
  }
  /**
   * @brief Pose constructs a new pose with given position and orientation
   * @param x the x coordinate of the translational part of the pose
   * @param y the y coordinate of the translational part of the pose
   * @param orientation an angle describing the orientation of the pose
   */
  Pose(const float x, const float y, const float orientation = 0)
    : position(x, y)
    , orientation(orientation)
  {
  }
  /**
   * @brief operator== compares this pose to anothe pose
   * @param other another pose
   * @return true if the poses are equal
   */
  bool operator==(const Pose& other) const
  {
    return position == other.position && orientation == other.orientation;
  }
  /**
   * @brief operator!= compares this pose to anothe pose
   * @param other another pose
   * @return false if the poses are equal
   */
  bool operator!=(const Pose& other) const
  {
    return !(*this == other);
  }
  /**
   * @brief operator+= sums another pose to this one.
   * @param pose the position that will be added.
   */
  Pose& operator+=(const Pose& other)
  {
    position = *this * other.position;
    orientation += other.orientation;
    return *this;
  }
  /**
   * @brief operator* transforms a position relative to this pose into a global one
   * @param other the position that is to be transformed
   */
  Vector2f operator*(const Vector2f& other) const
  {
    // This computes rot(orientation) * other + position.
    return calculateGlobalOrientation(other) + position;
  }
  /**
   * @brief operator* transforms a pose relative to this pose into a global one
   * @param other the pose that is to be transformed
   */
  Pose operator*(const Pose& other) const
  {
    return Pose(*this * other.position, orientation + other.orientation);
  }
  /**
   * @brief calculateGlobalOrientation rotates a Vector2 into global coordinates
   * @param other the vector to be rotated
   * @return the rotated vector
   */
  Vector2f calculateGlobalOrientation(const Vector2f& other) const
  {
    float cosine = std::cos(orientation);
    float sine = std::sin(orientation);
    return Vector2f(cosine * other.x() - sine * other.y(), sine * other.x() + cosine * other.y());
  }
  /**
   * @brief invert inverts the pose (i.e. the origin relative to the pose)
   * @return a pose that behaves like the inverse
   */
  Pose& invert()
  {
    orientation = -orientation;
    // This computes -(rot(orientation) * position).
    position = -calculateGlobalOrientation(position);
    return *this;
  }
  /**
   * @brief inverse computes the inverse but does not overwrite the existing object
   * @return a pose that behaves like the inverse
   */
  Pose inverse() const
  {
    return Pose(*this).invert();
  }
  /**
   * @brief toValue converts a pose into a Uni::Value
   * @param value where to store the converted value
   */
  void toValue(Uni::Value& value) const
  {
    value = Uni::Value(Uni::ValueType::ARRAY);
    value.reserve(2);
    value.at(0) << position;
    value.at(1) << orientation;
  }
  /**
   * @brief fromValue converts a Uni::Value into a pose
   * @param value a Uni::Value, e.g. from config
   */
  void fromValue(const Uni::Value& value)
  {
    assert(value.type() == Uni::ValueType::ARRAY);
    assert(value.size() == 2);
    value.at(0) >> position;
    value.at(1) >> orientation;
  }
  /// the position (meters)
  Vector2f position;
  /// the orientation (rad) increasing counterclockwise
  float orientation;
};
