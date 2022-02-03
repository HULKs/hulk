#include "Motion/Utils/DynamicMovementPrimitive/DynamicMovementPrimitive.hpp"
#include <cmath>


DynamicMovementPrimitive::DynamicMovementPrimitive(const float canonicalSystemFinalValue,
                                                   std::vector<float> weightingsAsVector)
  : canonicalTimeConstant_{-std::log(canonicalSystemFinalValue)}
  , springConstant_{canonicalTimeConstant_ * canonicalTimeConstant_}
  , dampingConstant_{2 * canonicalTimeConstant_}
  , weightingsAsVector_{std::move(weightingsAsVector)}
{
}

void DynamicMovementPrimitive::reset(const Vector3f& posInitial, const Vector3f& posFinal,
                                     const unsigned int duration)
{
  time_ = 0;
  duration_ = duration;

  /// initial and final values
  posInitial_ = posInitial;
  posFinal_ = posFinal;

  pos_ = posInitial_;
  vel_.setZero();
  acc_.setZero();

  numberOfBasisFunctions_ = static_cast<int>(weightingsAsVector_.size()) / 3;
  weightings_ =
      Eigen::Map<const Eigen::MatrixXf>(weightingsAsVector_.data(), 3, numberOfBasisFunctions_);
  centersTime_.resize(numberOfBasisFunctions_);
  centers_.resize(numberOfBasisFunctions_);
  widths_.resize(numberOfBasisFunctions_);
  basisFunctions_.resize(numberOfBasisFunctions_);

  for (int n = 0; n < numberOfBasisFunctions_; n++)
  {
    centersTime_(n) = static_cast<float>(duration_) * static_cast<float>(n) /
                      static_cast<float>(numberOfBasisFunctions_ - 1);
    centers_(n) =
        std::exp(-canonicalTimeConstant_ * centersTime_(n) / static_cast<float>(duration_));
    widths_(n) =
        static_cast<float>(numberOfBasisFunctions_) *
        std::exp(2 * canonicalTimeConstant_ * centersTime_(n) / static_cast<float>(duration_));
  }
}

Vector3f DynamicMovementPrimitive::step(const unsigned int dt)
{
  /// solution of canonical system
  float x =
      std::exp(-canonicalTimeConstant_ * static_cast<float>(time_) / static_cast<float>(duration_));

  /// calculate the nonlinearity f
  for (int n = 0; n < numberOfBasisFunctions_; n++)
  {
    basisFunctions_(n) = std::exp(-widths_(n) / 2 * (x - centers_(n)) * (x - centers_(n)));
  }
  Vector3f psiWeighted;
  psiWeighted = weightings_ * basisFunctions_;
  float psiSum = basisFunctions_.sum();
  Vector3f f = x / psiSum * psiWeighted;

  /// calculate acceleration
  acc_ = (springConstant_ * (posFinal_ - pos_) -
          dampingConstant_ * static_cast<float>(duration_) * vel_ -
          x * springConstant_ * (posFinal_ - posInitial_) + springConstant_ * f) /
         (duration_ * duration_);

  /// calculate velocity and position by integration
  pos_ += vel_ * dt;
  vel_ += acc_ * dt;

  time_ += dt;
  if (time_ > duration_)
  {
    time_ = duration_;
  }

  return pos_;
}

bool DynamicMovementPrimitive::finished() const
{
  return time_ >= duration_;
}
