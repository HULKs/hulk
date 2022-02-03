#pragma once

#include "Tools/Math/Eigen.hpp"
#include "Tools/Math/Hysteresis.hpp"
#include "Tools/Storage/UniValue/UniValue.h"

class Pose : public Uni::To, public Uni::From
{
public:
  Pose() = default;
  explicit Pose(Vector2f position);
  explicit Pose(Vector2f position, float orientation);
  Pose(float x, float y);
  Pose(float x, float y, float orientation);

  const float& x() const;
  float& x();
  const float& y() const;
  float& y();
  const Vector2f& position() const;
  Vector2f& position();
  const float& angle() const;
  float& angle();

  bool operator==(const Pose& other) const;
  bool operator!=(const Pose& other) const;
  /**
   * @brief operator*= sums another pose to this one.
   * @param pose the position that will be added.
   */
  Pose& operator*=(const Pose& other);
  /**
   * @brief operator* transforms a position relative to this pose into a global one
   * @param other the position that is to be transformed
   */
  Vector2f operator*(const Vector2f& other) const;
  /**
   * @brief operator* transforms a pose relative to this pose into a global one
   * @param other the pose that is to be transformed
   */
  Pose operator*(const Pose& other) const;
  /**
   * @brief operator/ scales the pose
   * @param scale the factor all components will be divided
   */
  Pose operator/(float scale) const;
  /*
   * @brief checks if another pose is similar to this one
   * @param other the Pose that should be checked for equality
   * @param positionThreshold the difference between the two positions has to be smaller than this
   * to be similar
   * @param orientationThreshold the difference between the orientations has to be smaller than this
   * to be similar
   * */
  bool isSimilar(const Pose& other, float positionThreshold, float orientationThreshold) const;
  /**
   * @brief calculateGlobalOrientation rotates a Vector2 into global coordinates
   * @param other the vector to be rotated
   * @return the rotated vector
   */
  Vector2f calculateGlobalOrientation(const Vector2f& other) const;
  /**
   * @brief invert inverts the pose (i.e. the origin relative to the pose)
   * @return a pose that behaves like the inverse
   */
  Pose& invert();
  /**
   * @brief inverse computes the inverse but does not overwrite the existing object
   * @return a pose that behaves like the inverse
   */
  Pose inverse() const;
  void toValue(Uni::Value& value) const override;
  void fromValue(const Uni::Value& value) override;

private:
  /// the position [meters]
  Vector2f position_{Vector2f::Zero()};
  /// the orientation [rad] increasing counterclockwise
  float angle_{0.f};
};


inline Pose::Pose(Vector2f position)
  : position_{std::move(position)}
{
}

inline Pose::Pose(Vector2f position, const float orientation)
  : position_{std::move(position)}
  , angle_{orientation}
{
}

inline Pose::Pose(const float x, const float y)
  : position_{x, y}
{
}

inline Pose::Pose(const float x, const float y, const float orientation)
  : position_{x, y}
  , angle_{orientation}
{
}

inline const float& Pose::x() const
{
  return position_.x();
}

inline float& Pose::x()
{
  return position_.x();
}

inline const float& Pose::y() const
{
  return position_.y();
}

inline float& Pose::y()
{
  return position_.y();
}

inline const Vector2f& Pose::position() const
{
  return position_;
}

inline Vector2f& Pose::position()
{
  return position_;
}

inline const float& Pose::angle() const
{
  return angle_;
}

inline float& Pose::angle()
{
  return angle_;
}

inline bool Pose::operator==(const Pose& other) const
{
  return position_ == other.position_ && angle_ == other.angle_;
}

inline bool Pose::operator!=(const Pose& other) const
{
  return !(*this == other);
}

inline Pose& Pose::operator*=(const Pose& other)
{
  position_ = *this * other.position_;
  angle_ += other.angle_;
  return *this;
}

inline Vector2f Pose::operator*(const Vector2f& other) const
{
  // This computes rot(orientation) * other + position.
  return calculateGlobalOrientation(other) + position_;
}

inline Pose Pose::operator*(const Pose& other) const
{
  return Pose{*this * other.position_, angle_ + other.angle_};
}

inline Pose Pose::operator/(const float scale) const
{
  return Pose{position_ / scale, angle_ / scale};
}

inline bool Pose::isSimilar(const Pose& other, const float positionThreshold,
                            const float orientationThreshold) const
{
  return (position_ - other.position_).norm() < positionThreshold &&
         Hysteresis::equalTo(angle_, other.angle_, orientationThreshold);
}

inline Vector2f Pose::calculateGlobalOrientation(const Vector2f& other) const
{
  float cosine = std::cos(angle_);
  float sine = std::sin(angle_);
  return Vector2f(cosine * other.x() - sine * other.y(), sine * other.x() + cosine * other.y());
}

inline Pose& Pose::invert()
{
  angle_ = -angle_;
  // This computes -(rot(orientation) * position).
  position_ = -calculateGlobalOrientation(position_);
  return *this;
}

inline Pose Pose::inverse() const
{
  return Pose(*this).invert();
}

inline void Pose::toValue(Uni::Value& value) const
{
  value = Uni::Value(Uni::ValueType::ARRAY);
  value.reserve(2);
  value.at(0) << position_;
  value.at(1) << angle_;
}

inline void Pose::fromValue(const Uni::Value& value)
{
  assert(value.type() == Uni::ValueType::ARRAY);
  assert(value.size() == 2);
  value.at(0) >> position_;
  value.at(1) >> angle_;
}
