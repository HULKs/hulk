#pragma once


#include "Tools/Math/Eigen.hpp"
#include "Tools/Storage/UniValue/UniConvertible.hpp"
#include <sstream>

/// Representation of Kinematic Information
/**
 * This class represents a KinematicMatrix
 * A KinematicMatrix is represented by a 3x3 RotationMatrix (rotM) and a Vector3 (posV)\n
 * \f{align*}{
 * \begin{bmatrix}
 *  rotM & posV \\
 *  0 &   1 \end{bmatrix}
 * \f}
 * The last row in a kinematic Matrix is always [ 0 0 0 1]
 * normally a KinematicMatrix should be of size 4x4, but because of the last row
 * the Matrix is only represented by a RotationMatrix and a PositionVector
 * @author <a href="mailto:stefan.kaufmann@tu-harburg.de">Stefan Kaufmann</a>
 */
class KinematicMatrix : public Uni::From, public Uni::To
{
public:
  /**
   * The RotationMatrix
   */
  AngleAxisf rotM;

  /**
   * The position vector
   */
  Vector3f posV;

  /** default constructor (creates Identity Matrix) */
  KinematicMatrix()
    : rotM(AngleAxisf::Identity())
    , posV(Vector3f::Zero())
  {
  }

  /** constructor with initialization of the RotationMatrix
   * @param rm the RotationMatrix
   */
  KinematicMatrix(const AngleAxisf& rm)
    : rotM(rm)
    , posV(Vector3f::Zero())
  {
  }

  /** constructor with initialization of the position-vector
   * @param p the position-vector
   */
  KinematicMatrix(const Vector3f& p)
    : rotM(AngleAxisf::Identity())
    , posV(p)
  {
  }

  /** constructor with initialization of the RotationMatrix and the position-vector
   * @param rm the RotationMatrix
   * @param p the position-vector
   */
  KinematicMatrix(const AngleAxisf& rm, const Vector3f& p)
    : rotM(rm)
    , posV(p)
  {
  }

  /** copy constructor
   * @param other the other KinematicMatrix
   */
  KinematicMatrix(const KinematicMatrix& other)
    : rotM(other.rotM)
    , posV(other.posV)
  {
  }

  /** inverts the KinematicMatrix
   * Note that because of the special structure, the inverse can be
   * calculated by
   * inv = |	inv(rotM)		-inv(RotM) * posV	|
   *		|		   0					    1	|
   *
   */
  KinematicMatrix invert() const
  {
    AngleAxisf invRot = rotM.inverse();
    Vector3f invPos = invRot * -posV;

    return KinematicMatrix(invRot, invPos);
  }

  /** creates a KinematicMatrix which represents a
   * rotation about the x-axis
   * @param alpha Angle of rotation
   */
  static KinematicMatrix rotX(const float& alpha)
  {
    return KinematicMatrix(AngleAxisf(alpha, Vector3f::UnitX()), Vector3f::Zero());
  }

  /** creates a KinematicMatrix which represents a
   * rotation about the y-axis
   * @param alpha Angle of rotation
   */
  static KinematicMatrix rotY(const float& alpha)
  {
    return KinematicMatrix(AngleAxisf(alpha, Vector3f::UnitY()), Vector3f::Zero());
  }

  /** creates a KinematicMatrix which represents a
   * rotation about the z-axis
   * @param alpha Angle of rotation
   */
  static KinematicMatrix rotZ(const float& alpha)
  {
    return KinematicMatrix(AngleAxisf(alpha, Vector3f::UnitZ()), Vector3f::Zero());
  }

  /** creates a KinematicMatrix which represents a
   * translation along the x-axis
   * @param distance Distance of translation
   */
  static KinematicMatrix transX(const float& distance)
  {
    return KinematicMatrix(AngleAxisf::Identity(), Vector3f(distance, 0, 0));
  }

  /** creates a KinematicMatrix which represents a
   * translation along the y-axis
   * @param distance Distance of translation
   */
  static KinematicMatrix transY(const float& distance)
  {
    return KinematicMatrix(AngleAxisf::Identity(), Vector3f(0, distance, 0));
  }

  /** creates a KinematicMatrix which represents a
   * translation along the z-axis
   * @param distance Distance of translation
   */
  static KinematicMatrix transZ(const float& distance)
  {
    return KinematicMatrix(AngleAxisf::Identity(), Vector3f(0, 0, distance));
  }

  /** multiplies a KinematicMatrix to this one
   * @param other other KinematicMatrix
   * @return product
   */
  KinematicMatrix& operator*=(const KinematicMatrix& other)
  {
    posV += rotM * other.posV;
    rotM = rotM * other.rotM;
    return *this;
  }

  /** multiplies a KinematicMatrix to another one
   * @param other other KinematicMatrix
   * @return product
   */
  KinematicMatrix operator*(const KinematicMatrix& other) const
  {
    return KinematicMatrix(*this) *= other;
  }

  /** comparison of another KinematicMatrix to this one
   * @param other KinematicMatrix
   * @return equality
   */
  bool operator==(const KinematicMatrix& other) const
  {
    return (rotM.isApprox(other.rotM) && posV.isApprox(other.posV));
  }

  /** comparison of another KinematicMatrix to this one
   * @param other KinematicMatrix
   * @return inequality
   */
  bool operator!=(const KinematicMatrix& other) const
  {
    return !(*this == other);
  }

  /** multiplication with a Vector3
   * This kind of multiplication allows to transformate coordinates
   * from one space to another
   * be careful: it is not a normal multiplication because of the special structure of Kinematic Matrices
   * @param position in source space
   * @return transformated position
   */
  Vector3f operator*(const Vector3f& position) const
  {
    return rotM * position + posV;
  }

  /**
   * Information of Matrix elements in a string.
   * Helpful for logging.
   */
  std::string toString()
  {
    std::ostringstream s;
    s << "Rotation: \n";
    s << rotM.toRotationMatrix() << "\n";
    s << "Position: \n";
    s << posV << "\n";

    return s.str();
  }

  void fromValue(const Uni::Value& value)
  {
    assert(value.type() == Uni::ValueType::ARRAY);
    assert(value.size() == 2);
    value.at(0) >> rotM;
    value.at(1) >> posV;
  }

  void toValue(Uni::Value& value) const
  {
    value = Uni::Value(Uni::ValueType::ARRAY);
    value.at(0) << rotM;
    value.at(1) << posV;
  }
};
