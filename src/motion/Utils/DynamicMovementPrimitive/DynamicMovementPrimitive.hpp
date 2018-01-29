#pragma once

#include "Tools/Math/Eigen.hpp"

/**
 * @brief
 * @author
 */
class DynamicMovementPrimitive
{
public:
  /**
   * @brief
   * @param canonicalSystemFinalValue
   * @param weightingsAsVector
   */
  DynamicMovementPrimitive(const float canonicalSystemFinalValue, const std::vector<float>& weightingsAsVector);

  /**
   *@brief
   * @param posInitial
   * @param posFinal
   * @param duration
   */
  void reset(const Vector3f& posInitial, const Vector3f& posFinal, const unsigned int duration);

  /**
   * @brief
   * @param dt
   * @return
   */
  Vector3f step(const unsigned int dt);

  /**
   * @brief checks if the DMP is done
   * @return true if done
   */
  bool finished();

private:
  unsigned int time_;

  float canonicalTimeConstant_;
  float springConstant_;
  float dampingConstant_;

  unsigned int duration_;
  Vector3f posInitial_;
  Vector3f posFinal_;

  Vector3f pos_;
  Vector3f vel_;
  Vector3f acc_;

  VectorXf basisFunctions_;
  VectorXf centersTime_;
  VectorXf centers_;
  VectorXf widths_;
  MatrixXf weightings_;
  int numberOfBasisFunctions_;

  std::vector<float> weightingsAsVector_;
};
