#include "Tools/Math/Angle.hpp"
#include "Tools/Math/Eigen.hpp"

#include "UKFPose2D.hpp"

void UKFPose2D::reset(const Vector3f& poseMean, const Vector3f& poseCov)
{
  stateMean_ = poseMean;

  stateCov_ = Matrix3f::Zero();
  stateCov_(0, 0) = poseCov(0);
  stateCov_(1, 1) = poseCov(1);
  stateCov_(2, 2) = poseCov(2);
  generateSigmaPoints();
}

void UKFPose2D::odometryPredict(const Pose& projectedTorsoMatrixChange,
                                const Vector3f& filterProcessNoise,
                                const Vector3f& odometryProcessNoise)
{
  // Generate the sigmaPoints_ for the UC predict:
  generateSigmaPoints();
  Vector3f lastStateMean = stateMean_;

  // Propagate each sigma point through nonlinear odometry predict
  for (auto& sigmaPoint : sigmaPoints_)
  {
    sigmaPoint += odometryToGlobal(projectedTorsoMatrixChange, sigmaPoint);
  }

  // compute mean an covariance of the deformed sigma point cloud
  float mX = 0.f;
  float mY = 0.f;
  Vector2f mDirection = Vector2f::Zero();

  for (auto& sigmaPoint : sigmaPoints_)
  {
    mX += sigmaPoint.x();
    mY += sigmaPoint.y();
    // special treatment for the angle as a circular quantity
    mDirection += Vector2f(std::cos(sigmaPoint.z()), std::sin(sigmaPoint.z()));
  }
  mX *= 1.f / 7.f;
  mY *= 1.f / 7.f;
  mDirection *= 1.f / 7.f;
  stateMean_ = Vector3f(mX, mY, atan2(mDirection.y(), mDirection.x()));

  // covariance
  stateCov_ = Matrix3f::Zero();
  for (auto& sigmaPoint : sigmaPoints_)
  {
    Vector3f diff = sigmaPoint - stateMean_;
    diff.z() = Angle::normalizeAngleDiff(diff.z());
    stateCov_ += (Matrix3f() << diff * diff.x(), diff * diff.y(), diff * diff.z()).finished();
  }
  stateCov_ *= 0.5f;
  fixCovariance(stateCov_);

  // The process noise (usually integrated in the "augmented state") is added manually
  // Since the predict noise depends on the orienation, the noise needs to be transformed
  // accordingly
  Matrix3f robot2fieldRotation;
  robot2fieldRotation = AngleAxisf(stateMean_.z(), Vector3f::UnitZ());
  Vector3f rotatedOdometryProcessNoise = robot2fieldRotation * odometryProcessNoise;

  Vector3f stateMeanChange = stateMean_ - lastStateMean;
  for (int i = 0; i < 3; i++)
  {
    stateCov_(i, i) += filterProcessNoise(i);
    stateCov_(i, i) += std::fabs(rotatedOdometryProcessNoise(i) * stateMeanChange(i));
  }

  // assert for symmetric covariance to avoid devergence of the filter
  assert(stateCov_(0, 1) == stateCov_(1, 0) && stateCov_(0, 2) == stateCov_(2, 0) &&
         stateCov_(1, 2) == stateCov_(2, 1));

  // normalize the angle of the state mean
  stateMean_.z() = Angle::normalized(stateMean_.z());
}

void UKFPose2D::generateSigmaPoints()
{
  // Cholesky decomposition
  Eigen::LLT<Matrix3f> choleskyDecomposition(stateCov_);
  const Matrix3f mSquareRoot = choleskyDecomposition.matrixL();

  // Compute representative sample
  sigmaPoints_[0] = stateMean_;
  sigmaPoints_[1] = stateMean_ + mSquareRoot.col(0);
  sigmaPoints_[2] = stateMean_ - mSquareRoot.col(0);
  sigmaPoints_[3] = stateMean_ + mSquareRoot.col(1);
  sigmaPoints_[4] = stateMean_ - mSquareRoot.col(1);
  sigmaPoints_[5] = stateMean_ + mSquareRoot.col(2);
  sigmaPoints_[6] = stateMean_ - mSquareRoot.col(2);
}

void UKFPose2D::poseSensorUpdate(const Vector3f& poseObservation, const Matrix3f& covObservation)
{
  // first generate the sigma points:
  generateSigmaPoints();

  // the sigmaPoints are propagated through the observation function h(sigmaPoint) into the space of
  // observation this produces the predicted observation. Here h is the identity matrix, thus the
  // predictedObservations match the sigmaPoints_
  std::array<Vector3f, 7> predictedObservations = sigmaPoints_;

  // compute mean of predicted observations:
  Vector3f predictedObservationMean = Vector3f::Zero();
  float mX = 0;
  float mY = 0;
  Vector2f mDirection = Vector2f::Zero();

  for (auto& predictedObservation : predictedObservations)
  {
    mX += predictedObservation.x();
    mY += predictedObservation.y();
    mDirection += Vector2f(std::cos(predictedObservation.z()), std::sin(predictedObservation.z()));
  }
  mX *= 1.f / 7.f;
  mY *= 1.f / 7.f;
  mDirection *= 1.f / 7.f;
  predictedObservationMean = Vector3f(mX, mY, atan2(mDirection.y(), mDirection.x()));
  predictedObservationMean.z() = Angle::normalized(predictedObservationMean.z());

  // Pzz - compute covariance of predicted observations - as computed above:
  Matrix3f predictedObservationsCov = Matrix3f::Zero();
  for (auto& predictedObservation : predictedObservations)
  {
    Vector3f diff = predictedObservation - predictedObservationMean;
    diff.z() = Angle::normalizeAngleDiff(diff.z());
    predictedObservationsCov +=
        (Matrix3f() << diff * diff.x(), diff * diff.y(), diff * diff.z()).finished();
  }
  predictedObservationsCov *= 0.5f;

  // Pxz - cross-covaraince matrix of sigma Points and observations
  Matrix3f predictedObservationsCrossCov = Matrix3f::Zero();
  for (int i = 0; i < 7; i++)
  {
    Vector3f diffX = sigmaPoints_[i] - stateMean_;
    diffX.z() = Angle::normalizeAngleDiff(diffX.z());
    Vector3f diffZ = predictedObservations[i] - predictedObservationMean;
    diffZ.z() = Angle::normalizeAngleDiff(diffZ.z());
    predictedObservationsCrossCov += diffX * diffZ.transpose();
  }
  predictedObservationsCrossCov *= 0.5f;

  // compute the UKF Kalman gain
  const Matrix3f kalmanGain =
      predictedObservationsCrossCov * (predictedObservationsCov + covObservation).inverse();

  // residuum
  Vector3f residuum = poseObservation - predictedObservationMean;
  residuum.z() = Angle::normalizeAngleDiff(residuum.z());

  // a posteriori state estimate
  stateMean_ += kalmanGain * residuum;
  stateMean_.z() = Angle::normalized(stateMean_.z());

  // a posterior state covariance
  stateCov_ -= kalmanGain * predictedObservationsCrossCov.transpose();
  fixCovariance(stateCov_);
}

void UKFPose2D::pose1DSensorUpdate(const Vector2f& pose1DObservation, const bool updateXDirection,
                                   const Matrix2f& distAndAngleCov)
{
  // first generate the sigma points:
  generateSigmaPoints();

  // the sigmaPoints are propagated through the observation function h(sigmaPoint) into the space of
  // observation this produces the predicted observation. Here h is a map, pruning the dimension
  // along which the line runs
  std::array<Vector2f, 7> predictedObservations;
  if (updateXDirection)
  {
    // The space of observation for a line along y constits of: (x-position, orientation)
    for (int i = 0; i < 7; i++)
    {
      predictedObservations[i] = Vector2f(sigmaPoints_[i].x(), sigmaPoints_[i].z());
    }
  }
  else
  {
    // The space of observation for a line along x constits of: (y-position, orientation)
    for (int i = 0; i < 7; i++)
    {
      predictedObservations[i] = Vector2f(sigmaPoints_[i].y(), sigmaPoints_[i].z());
    }
  }

  // compute mean of predicted observations:
  // TODO: this should be obsolete due to the fact that h is the (pruning) identity (maybe log the
  // diff to pruned stateMean_)
  Vector2f predictedObservationMean = Vector2f::Zero();
  float mCoordinate = 0;
  Vector2f mDirection = Vector2f::Zero();

  for (auto& predictedObservation : predictedObservations)
  {
    mCoordinate += predictedObservation.x();
    mDirection += Vector2f(std::cos(predictedObservation.y()), std::sin(predictedObservation.y()));
  }

  mCoordinate *= 1.f / 7.f;
  mDirection *= 1.f / 7.f;
  predictedObservationMean = Vector2f(mCoordinate, atan2(mDirection.y(), mDirection.x()));
  predictedObservationMean.y() = Angle::normalized(predictedObservationMean.y());

  // Pzz - compute covariance of predicted observations - as computed above:
  Matrix2f predictedObservationsCov = Matrix2f::Zero();
  for (auto& predictedObservation : predictedObservations)
  {
    Vector2f diff = predictedObservation - predictedObservationMean;
    diff.y() = Angle::normalizeAngleDiff(diff.y());
    predictedObservationsCov += (Matrix2f() << diff * diff.x(), diff * diff.y()).finished();
  }
  predictedObservationsCov *= 0.5f;

  // Pxz - cross-covaraince matrix of sigma Points and observations
  Eigen::Matrix<float, 3, 2> predictedObservationsCrossCov = Eigen::Matrix<float, 3, 2>::Zero();
  for (int i = 0; i < 7; i++)
  {
    Vector3f diffX = sigmaPoints_[i] - stateMean_;
    diffX.z() = Angle::normalizeAngleDiff(diffX.z());
    Vector2f diffZ = predictedObservations[i] - predictedObservationMean;
    diffZ.y() = Angle::normalizeAngleDiff(diffZ.y());
    predictedObservationsCrossCov += diffX * diffZ.transpose();
  }
  predictedObservationsCrossCov *= 0.5f;

  // compute the UKF Kalman gain
  const Eigen::Matrix<float, 3, 2> kalmanGain =
      predictedObservationsCrossCov * (predictedObservationsCov + distAndAngleCov).inverse();

  // residuum
  Vector2f residuum = pose1DObservation - predictedObservationMean;
  residuum.y() = Angle::normalizeAngleDiff(residuum.y());

  // a posteriori state estimate
  stateMean_ += kalmanGain * residuum;
  stateMean_.z() = Angle::normalized(stateMean_.z());

  // a posterior state covariance
  // xk = x_k + K * Pzz * K^T
  // xk = x_k + Pxz*Pzz^(-1)*Pzz^(-1)^T*Pxz^T | Pzz^(-1)^T = Pzz^(-1)
  // xk = x_k + Pxz*Pzz^(-1) * Pxz^T          | Pxz*Pzz^(-1) = K
  // xk = x_k + K*Pxz^T
  stateCov_ -= kalmanGain * predictedObservationsCrossCov.transpose();
  fixCovariance(stateCov_);
}

void UKFPose2D::fieldPointUpdate(const Vector2f& relativeFieldPoint,
                                 const Vector2f& absoluteFieldPointPosition,
                                 const Matrix2f& covObservation)
{
  // first generate the sigma points:
  generateSigmaPoints();

  // the sigmaPoints are propagated through the observation function h(sigmaPoint) into the space of
  // observation this produces the predicted observation. Here h maps to the the
  // absoluteFieldPointPosition to the corresponding relative position
  std::array<Vector2f, 7> predictedObservations;
  for (int i = 0; i < 7; i++)
  {
    // Where would the field mark be in relative coordinates of this sigma point:
    const Rotation2Df sigmaPointRotation(sigmaPoints_[i].z());
    const Vector2f absoluteSigmaPointPosition(sigmaPoints_[i].x(), sigmaPoints_[i].y());
    predictedObservations[i] =
        sigmaPointRotation.inverse() * (absoluteFieldPointPosition - absoluteSigmaPointPosition);
  }

  // compute mean of predicted observations:
  Vector2f predictedObservationMean = Vector2f::Zero();
  float mX = 0;
  float mY = 0;

  for (auto& predictedObservation : predictedObservations)
  {
    mX += predictedObservation.x();
    mY += predictedObservation.y();
  }
  mX *= 1.f / 7.f;
  mY *= 1.f / 7.f;
  predictedObservationMean = Vector2f(mX, mY);

  // Pzz - compute covariance of predicted observations - as computed above:
  Matrix2f predictedObservationsCov = Matrix2f::Zero();
  for (auto& predictedObservation : predictedObservations)
  {
    Vector2f diff = predictedObservation - predictedObservationMean;
    predictedObservationsCov += (Matrix2f() << diff * diff.x(), diff * diff.y()).finished();
  }
  predictedObservationsCov *= 0.5f;

  // Pxz - cross-covaraince matrix of sigma Points and observations
  Eigen::Matrix<float, 3, 2> predictedObservationsCrossCov = Eigen::Matrix<float, 3, 2>::Zero();
  for (int i = 0; i < 7; i++)
  {
    Vector3f diffX = sigmaPoints_[i] - stateMean_;
    diffX.z() = Angle::normalizeAngleDiff(diffX.z());
    Vector2f diffZ = predictedObservations[i] - predictedObservationMean;
    predictedObservationsCrossCov += diffX * diffZ.transpose();
  }
  predictedObservationsCrossCov *= 0.5f;

  // compute the UKF Kalman gain
  const Eigen::Matrix<float, 3, 2> kalmanGain =
      predictedObservationsCrossCov * (predictedObservationsCov + covObservation).inverse();

  // residuum
  Vector2f residuum = relativeFieldPoint - predictedObservationMean;

  // a posteriori state estimate
  stateMean_ += kalmanGain * residuum;
  stateMean_.z() = Angle::normalized(stateMean_.z());

  // a posterior state covariance
  stateCov_ -= kalmanGain * predictedObservationsCrossCov.transpose();
  fixCovariance(stateCov_);
}

float UKFPose2D::hesseNormalDist(const Line<float>& line, const Vector2f& point) const
{
  const float l2 = (line.p2 - line.p1).squaredNorm();
  if (l2 == 0.0)
  {
    return (point - line.p1).norm();
  }

  // Consider the line extending the segment, parameterized as p1 + t * (p2 - p1).
  // We find projection of point "point" onto the line.
  // It falls where t = [(p - p1) . (p2 - p1)] / |p2 - p1|^2

  const float t = (point - line.p1).dot(line.p2 - line.p1) / l2;
  const Vector2f projection = line.p1 + (line.p2 - line.p1) * t;

  float sign = ((line.p2.x() - line.p1.x()) * (point.y() - line.p1.y()) -
                (point.x() - line.p1.x()) * (line.p2.y() - line.p1.y()));
  if (sign > 0.f)
  { // point is left of line (if drawn from p1 to p2)
    sign = 1.f;
  }
  else if (sign < 0.f)
  { // point is right of line
    sign = -1.f;
  }

  return sign * (point - projection).norm();
}

Vector3f UKFPose2D::odometryToGlobal(const Pose& relativeOdometeryPredict,
                                     const Vector3f& referencePose) const
{
  float c = std::cos(referencePose.z());
  float s = std::sin(referencePose.z());
  return Vector3f(
      c * relativeOdometeryPredict.position.x() - s * relativeOdometeryPredict.position.y(),
      s * relativeOdometeryPredict.position.x() + c * relativeOdometeryPredict.position.y(),
      relativeOdometeryPredict.orientation);
}

void UKFPose2D::fixCovariance(Matrix2f& cov) const
{
  cov(0, 1) = (cov(0, 1) + cov(1, 0)) * .5f;
  cov(1, 0) = cov(0, 1);
}

void UKFPose2D::fixCovariance(Matrix3f& cov) const
{
  cov(0, 1) = (cov(0, 1) + cov(1, 0)) * .5f;
  cov(1, 0) = cov(0, 1);

  cov(1, 2) = (cov(1, 2) + cov(2, 1)) * .5f;
  cov(2, 1) = cov(1, 2);

  cov(0, 2) = (cov(0, 2) + cov(2, 0)) * .5f;
  cov(2, 0) = cov(0, 2);
}
