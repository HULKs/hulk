#pragma once

#include "Data/OdometryOffset.hpp"
#include "Tools/Math/Eigen.hpp"
#include "Tools/Math/Line.hpp"
#include "Tools/Math/Pose.hpp"
#include "Tools/Storage/UniValue/UniConvertible.hpp"
#include <cmath>

class UKFPose2D : public Uni::To
{
public:
  /**
   * @brief UKFPose2D creates a new UKFPose2D
   */
  UKFPose2D() {}
  /**
   * @brief generateSigmaPoints Generates the sigmaPoints_ based on the state distribution function.
   */
  void generateSigmaPoints();
  /**
   * @brief reset resets UKFPose to given mean and covariance
   * @param poseMean the state mean to be set
   * @param poseCov the diagonal entries of the intial cov matrix
   */
  void reset(const Vector3f& poseMean, const Vector3f& poseCov);
  /**
   * @brief odometryPredict performs the UC predict with a odometry data
   * @param projectedTorsoMatrixChange the projected torso matrix change
   * @param filterProcessNoise the noise of of the filter process
   * @param odometryProcessNoise the noise of the odometry change
   */
  void odometryPredict(const Pose& projectedTorsoMatrixChange, const Vector3f& filterProcessNoise,
                       const Vector3f& odometryProcessNoise);
  /**
   * @brief poseSensorUpdate performs an UC update of the UKF with a full pose (x, y, alpha)
   * @param poseObservation the pose update (x, y, alpha)
   * @param covObservation the covariance of the observed pose
   */
  void poseSensorUpdate(const Vector3f& poseObservation, const Matrix3f& covObservation);
  /**
   * @brief pose1DSensorUpdate performs an UC update of the UKF with a 1D-Pose (position along an
   * axis and given orientation)
   * @brief updateXDirection true if the x-direction is to be updated (e.g. resulting from a line
   * along y-axis);
   * @param pose1DObservation the pose update (absolute cooradinates, (x/y, orienation)) to use for
   * the update
   * @param distAndAngleCov the covariance of distance and orientation of the measurement
   */
  void pose1DSensorUpdate(const Vector2f& pose1DObservation, const bool updateXDirectoin,
                          const Matrix2f& distAndAngleCov);
  /**
   * @brief fieldPointUpdate performs an UC update of the UKF with a given relative field mark and
   * the associated point in absolute coordinates
   * @param relativeFieldPoint the position of the field point relative to this robot
   * @param aboluteFieldPointPosition the absolute position of the associated field point
   * @param covObservation the covariance of the fieldPoint measurement
   */
  void fieldPointUpdate(const Vector2f& relativeFieldPoint,
                        const Vector2f& absoluteFieldPointPosition, const Matrix2f& covObservation);
  /**
   * @brief getPoseMean making the stateMean accessable
   * @return the current pose resulting form the stateMean
   */
  Pose getPoseMean() const
  {
    return Pose(stateMean_.x(), stateMean_.y(), stateMean_.z());
  }
  /**
   * @brief getStateMean getter for the state mean
   * @return a const reference to the state mean
   */
  const auto& getStateMean() const
  {
    return stateMean_;
  }
  /**
   * @brief getPoseCov making the stateCov accessable
   * @return a reference to the stateCovariance
   */
  const auto& getStateCov() const
  {
    return stateCov_;
  }
  /**
   * @brief toValue converts this to a Uni::Value
   * @param value the resulting Uni::Value
   */
  virtual void toValue(Uni::Value& value) const
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["stateMean"] << stateMean_;
    value["stateCov"] << stateCov_;
    value["sigmaPoints"] << sigmaPoints_;
  }

protected:
  /// the mean of the random variable (RV) - (x, y, orientation)
  Vector3f stateMean_ = Vector3f::Zero();
  /// the COV of the RV
  Matrix3f stateCov_ = Matrix3f::Zero();
  /// the sigma points - a minimal set of representive samples
  std::array<Vector3f, 7> sigmaPoints_;

  /**
   * @brief hesseNormalDist calculates the signed distance of a point to a line (result > 0 <=>
   * point left of Vector(line.p2 - line.p1))
   * @param line a reference to a Line the distance is calculated to
   * @param point a point whichs distance to the given line above is to be calculated
   * @return the signed distance between the point and the line
   */
  float hesseNormalDist(const Line<float>& line, const Vector2f& point) const;
  /**
   * @brief transformToGlobal transforms a relativePose (in coordinates of a reference pose) to
   * global coordinates
   * @param relativePose the relative pose, to be transformed to the global coordinate system of the
   * reference pose
   * @param referencePose the referePose in whichs coordinates the relativePose is given
   * @return the tranformed pose
   */
  Vector3f odometryToGlobal(const Pose& relativeOdometryPredict,
                            const Vector3f& referencePose) const;
  /**
   * @brief fixCovariance ensures that a given cov is symmetric
   * @param cov a reference to the covariance matrix to be fixed
   */
  void fixCovariance(Matrix2f& cov) const;
  void fixCovariance(Matrix3f& cov) const;
};
