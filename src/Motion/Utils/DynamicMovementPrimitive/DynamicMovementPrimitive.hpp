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
  DynamicMovementPrimitive(float canonicalSystemFinalValue, std::vector<float> weightingsAsVector);

  /**
   *@brief
   * @param posInitial
   * @param posFinal
   * @param duration
   */
  void reset(const Vector3f& posInitial, const Vector3f& posFinal, unsigned int duration);

  /**
   * @brief
   * @param dt
   * @return
   */
  Vector3f step(unsigned int dt);

  /**
   * @brief checks if the DMP is done
   * @return true if done
   */
  bool finished() const;

private:
  unsigned int time_{0};

  float canonicalTimeConstant_{0.f};
  float springConstant_{0.f};
  float dampingConstant_{0.f};

  unsigned int duration_{0};
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
  int numberOfBasisFunctions_{0};

  std::vector<float> weightingsAsVector_;
};
