#pragma once

#include "Covariance.hpp"
#include "Tools/Kinematics/KinematicMatrix.h"
#include "Tools/Math/Angle.hpp"
#include "Tools/Math/Eigen.hpp"

class ProjectionMeasurementModel
{
public:
  /**
   * @brief ProjectionMeasurementModel a model for the uncertainty of projected measurments
   */
  ProjectionMeasurementModel()
    : cameraRPYDeviation_(5, 15, 2)
    , measurementBaseVariance_(0.5, 0.5)
  {
  }
  /**
   * @brief ProjectionMeasurementModel a model for the uncertainty of projected measurments
   * @param cameraRPYDeviation the deviation of the cameras roll, pitch and yaw
   * @param measurementBaseVariance the measurement variance (diagonal entries of the covariance
   * matrix) that is always present
   */
  ProjectionMeasurementModel(const Vector3f& cameraRPYDeviation,
                             const Vector2f& measurementBaseVariance)
    : cameraRPYDeviation_(cameraRPYDeviation)
    , measurementBaseVariance_(measurementBaseVariance)
  {
  }
  /**
   * @brief resetParameters resets the parameters to new given values
   * @param cameraRPYDeviation the deviation of the cameras roll, pitch and yaw
   * @param measurementBaseVariance the measurement variance (diagonal entries of the covariance
   */
  void resetParameters(const Vector3f& cameraRPYDeviation, const Vector2f& measurementBaseVariance)
  {
    cameraRPYDeviation_ = cameraRPYDeviation;
    measurementBaseVariance_ = measurementBaseVariance;
  }
  /**
   * @brief computeCovFromErrorPropagation
   * @param refPoint a point (on the ground) the covariance referrs to (target of projection)
   * @param cam2ground the camera matrix this projection was calculated with
   * @return the covariance of this point obtained from the propagated projection error
   */
  Matrix2f computeCovFromErrorPropagation(const Vector2f& refPoint,
                                          const KinematicMatrix& cam2ground) const
  {
    Vector2f deviationRollPitch = {cameraRPYDeviation_.x(),
                                   cameraRPYDeviation_.y()}; // Pitch uncertainty

    float distanceOnGround = refPoint.norm();
    Matrix2f rotPointVector2Robot;

    Vector2f refPointNormalized = refPoint / distanceOnGround;
    if (distanceOnGround < 0.000001f)
    {
      rotPointVector2Robot = Matrix2f::Identity();
    }
    else
    {
      rotPointVector2Robot << refPointNormalized.x(), -refPointNormalized.y(),
          refPointNormalized.y(), refPointNormalized.x();
    }
    // Transform the deviation in to the according coordinates
    deviationRollPitch = rotPointVector2Robot.transpose() * deviationRollPitch;
    /**
     * How does a pitch error Ep influence the distance error Ed
     * Looking at the observation function:
     * d = h * tan(phi), Ephi ~ Ep
     * One can obtain the propagated uncertainty (tailor series expansion)
     * Ed = d/dphi (h * tan(phi)) * Ep
     * Ed = 2 * h / (cos(2*phi) + 1) * Ep
     * From the tailor series one can obtain the error propagation
     */
    const float heightOverGround = cam2ground.posV.z();
    const float phi = std::atan(distanceOnGround / heightOverGround);
    float Ed = std::abs(2 * heightOverGround / (std::cos(2 * phi) + 1) * deviationRollPitch.y());
    Ed = std::isnan(Ed) ? 1337.f : Ed;

    // sideways error
    const float Es = std::abs(std::tan(cameraRPYDeviation_.z()) * distanceOnGround) +
                     std::abs(heightOverGround * std::tan(deviationRollPitch.x()));

    Matrix2f pointCov;
    pointCov << Ed * Ed, 0.f, 0.f, Es * Es;
    // Transform the covariance to robot coordinates
    pointCov = rotPointVector2Robot * pointCov * rotPointVector2Robot.transpose();
    // fix covariance in case of numric errors
    Covariance::fixCovariance(pointCov);

    return pointCov;
  }
  /**
   * @brief computePointCovFromPositionFeature calculates the covariance of a feature that
   * contains information about the x and y components of the state
   * @param relativePoint the percepted point feature in robot coordinates
   * @param cam2ground the relative 3D pose of the camera with respect to the ground
   * @return the resulting covariance estimation
   */
  Eigen::Matrix2f computePointCovFromPositionFeature(const Vector2f& relativePoint,
                                                     const KinematicMatrix& cam2ground) const
  {
    const Eigen::Matrix2f dynamicCov = computeCovFromErrorPropagation(relativePoint, cam2ground);
    return Eigen::DiagonalMatrix<float, 2>(measurementBaseVariance_.x() + dynamicCov(0, 0),
                                           measurementBaseVariance_.y() + dynamicCov(1, 1));
  }

private:
  /// the deviation of the cameras roll, pitch and yaw
  Vector3f cameraRPYDeviation_;
  /// the base variance that is assumed for all measurements
  Vector2f measurementBaseVariance_;
};
