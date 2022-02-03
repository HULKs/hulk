#pragma once

#include "Tools/Math/Eigen.hpp"
#include "Tools/Storage/UniValue/UniValue.h"
#include <utility>

/// Representation of Kinematic Information
/**
 * @brief This class represents a KinematicMatrix
 * A KinematicMatrix is represented by a 3x3 RotationMatrix (rotM) and a Vector3 (posV)
 * The last row in a kinematic Matrix is always [ 0 0 0 1]
 * normally a KinematicMatrix should be of size 4x4, but because of the last row
 * the Matrix is only represented by a RotationMatrix and a PositionVector
 */
class KinematicMatrix : public Uni::From, public Uni::To
{
public:
  /// The RotationMatrix
  AngleAxisf rotM{AngleAxisf::Identity()};

  /// The position vector
  Vector3f posV{Vector3f::Zero()};

  /**
   * @brief default constructor (creates Identity Matrix)
   */
  KinematicMatrix() = default;

  /*
   * @brief constructor with initialization of the RotationMatrix
   * @param rm the RotationMatrix
   */
  explicit KinematicMatrix(AngleAxisf rm);

  /**
   * @brief constructor with initialization of the position-vector
   * @param p the position-vector
   */
  explicit KinematicMatrix(Vector3f p);

  /**
   * @brief constructor with initialization of the RotationMatrix and the position-vector
   * @param rm the RotationMatrix
   * @param p the position-vector
   */
  KinematicMatrix(AngleAxisf rm, Vector3f p);

  /**
   * @brief returns the inverted KinematicMatrix of this
   * @return the inverted KinematicMatrix
   */
  KinematicMatrix inverted() const;

  /**
   * @brief creates a KinematicMatrix which represents a rotation about the x-axis
   * @param alpha Angle of rotation
   */
  static KinematicMatrix rotX(float alpha);

  /**
   * @brief creates a KinematicMatrix which represents a rotation about the y-axis
   * @param alpha Angle of rotation
   */
  static KinematicMatrix rotY(float alpha);

  /**
   * @brief creates a KinematicMatrix which represents a rotation about the z-axis
   * @param alpha Angle of rotation
   */
  static KinematicMatrix rotZ(float alpha);

  /**
   * @brief creates a KinematicMatrix which represents a translation along the x-axis
   * @param distance Distance of translation
   */
  static KinematicMatrix transX(float distance);

  /**
   * @brief creates a KinematicMatrix which represents a translation along the y-axis
   * @param distance Distance of translation
   */
  static KinematicMatrix transY(float distance);

  /**
   * @brief creates a KinematicMatrix which represents a translation along the z-axis
   * @param distance Distance of translation
   */
  static KinematicMatrix transZ(float distance);

  /**
   * @brief multiplies a KinematicMatrix to this one
   * @param other other KinematicMatrix
   * @return product
   */
  KinematicMatrix& operator*=(const KinematicMatrix& other);

  /**
   * @brief multiplies a KinematicMatrix to another one
   * @param other other KinematicMatrix
   * @return product
   */
  KinematicMatrix operator*(const KinematicMatrix& other) const;

  /**
   * @brief comparison of another KinematicMatrix to this one
   * @param other KinematicMatrix
   * @return equality
   */
  bool operator==(const KinematicMatrix& other) const;

  /**
   * @brief comparison of another KinematicMatrix to this one
   * @param other KinematicMatrix
   * @return inequality
   */
  bool operator!=(const KinematicMatrix& other) const;

  /**
   * @brief multiplication with a Vector3
   *
   * This kind of multiplication allows to transformate coordinates
   * from one space to another
   * be careful: it is not a normal multiplication because of the special structure of Kinematic
   * Matrices
   *
   * @param position in source space
   * @return transformated position
   */
  Vector3f operator*(const Vector3f& position) const;

  void fromValue(const Uni::Value& value) override;
  void toValue(Uni::Value& value) const override;
};


inline KinematicMatrix::KinematicMatrix(AngleAxisf rm)
  : rotM(std::move(rm))
{
}

inline KinematicMatrix::KinematicMatrix(Vector3f p)
  : posV(std::move(p))
{
}

inline KinematicMatrix::KinematicMatrix(AngleAxisf rm, Vector3f p)
  : rotM(std::move(rm))
  , posV(std::move(p))
{
}

inline KinematicMatrix KinematicMatrix::inverted() const
{
  const AngleAxisf invRot = rotM.inverse();
  const Vector3f invPos = invRot * -posV;
  return KinematicMatrix(invRot, invPos);
}

inline KinematicMatrix KinematicMatrix::rotX(float alpha)
{
  return KinematicMatrix(AngleAxisf(alpha, Vector3f::UnitX()), Vector3f::Zero());
}

inline KinematicMatrix KinematicMatrix::rotY(float alpha)
{
  return KinematicMatrix(AngleAxisf(alpha, Vector3f::UnitY()), Vector3f::Zero());
}

inline KinematicMatrix KinematicMatrix::rotZ(float alpha)
{
  return KinematicMatrix(AngleAxisf(alpha, Vector3f::UnitZ()), Vector3f::Zero());
}

inline KinematicMatrix KinematicMatrix::transX(float distance)
{
  return KinematicMatrix(AngleAxisf::Identity(), Vector3f(distance, 0, 0));
}

inline KinematicMatrix KinematicMatrix::transY(float distance)
{
  return KinematicMatrix(AngleAxisf::Identity(), Vector3f(0, distance, 0));
}

inline KinematicMatrix KinematicMatrix::transZ(float distance)
{
  return KinematicMatrix(AngleAxisf::Identity(), Vector3f(0, 0, distance));
}

inline KinematicMatrix& KinematicMatrix::operator*=(const KinematicMatrix& other)
{
  posV += rotM * other.posV;
  rotM = rotM * other.rotM;
  return *this;
}

inline KinematicMatrix KinematicMatrix::operator*(const KinematicMatrix& other) const
{
  return KinematicMatrix(*this) *= other;
}

inline bool KinematicMatrix::operator==(const KinematicMatrix& other) const
{
  return rotM.isApprox(other.rotM) && posV.isApprox(other.posV);
}

inline bool KinematicMatrix::operator!=(const KinematicMatrix& other) const
{
  return !(*this == other);
}

inline Vector3f KinematicMatrix::operator*(const Vector3f& position) const
{
  return rotM * position + posV;
}


inline void KinematicMatrix::fromValue(const Uni::Value& value)
{
  assert(value.type() == Uni::ValueType::ARRAY);
  assert(value.size() == 2);
  value.at(0) >> rotM;
  value.at(1) >> posV;
}

inline void KinematicMatrix::toValue(Uni::Value& value) const
{
  value = Uni::Value(Uni::ValueType::ARRAY);
  value.at(0) << rotM;
  value.at(1) << posV;
}
