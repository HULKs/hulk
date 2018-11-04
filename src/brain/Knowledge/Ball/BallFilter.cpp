#include <algorithm>
#include <limits>

#include <Modules/NaoProvider.h>
#include <Tools/Math/Random.hpp>
#include <Tools/Time.hpp>

#include "BallFilter.hpp"


BallFilter::BallFilter(const ModuleManagerInterface& manager)
  : Module(manager)
  , restingProcessCovX_(*this, "restingProcessCovX", [] {})
  , movingProcessCovX_(*this, "movingProcessCovX", [] {})
  , movingProcessCovDxX_(*this, "movingProcessCovDxX", [] {})
  , movingProcessCovDx_(*this, "movingProcessCovDx", [] {})
  , measurementBaseVariance_(*this, "measurementBaseVariance",
                             [this] {
                               projectionMeasurementModel_.resetParameters(
                                   cameraRPYDeviation_(), measurementBaseVariance_());
                             })
  , cameraRPYDeviation_(*this, "cameraRPYDeviation",
                        [this] {
                          cameraRPYDeviation_() *= TO_RAD;
                          projectionMeasurementModel_.resetParameters(cameraRPYDeviation_(),
                                                                      measurementBaseVariance_());
                        })
  , maxAssociationDistance_(*this, "maxAssociationDistance", [] {})
  , ballFrictionMu_(*this, "ballFrictionMu",
                    [this] { frictionDeceleration_ = 9.81f * ballFrictionMu_(); })
  , relativeMovingThreshold_(*this, "relativeMovingThreshold", [] {})
  , restingErrorLowPassAlpha_(*this, "restingErrorLowPassAlpha", [] {})
  , movingErrorLowPassAlpha_(*this, "movingErrorLowPassAlpha", [] {})
  , maxRestingError_(*this, "maxRestingError", [] {})
  , numOfRestingDeccelerationSteps_(*this, "numOfRestingDeccelerationSteps", [] {})
  , confidentMeasurementThreshold_(*this, "confidentMeasurementThreshold", [] {})
  , playerConfiguration_(*this)
  , ballData_(*this)
  , fieldDimensions_(*this)
  , odometryOffset_(*this)
  , cameraMatrix_(*this)
  , cycleInfo_(*this)
  , frictionDeceleration_(9.81f * ballFrictionMu_())
  , ballState_(*this)
  , ballModes_()
  , bestMode_(ballModes_.end())
  , projectionMeasurementModel_()
{
  cameraRPYDeviation_() *= TO_RAD;
  projectionMeasurementModel_.resetParameters(cameraRPYDeviation_(), measurementBaseVariance_());
}

void BallFilter::cycle()
{
  predict();
  if (ballData_->timestamp != lastTimestamp_)
  {
    lastTimestamp_ = ballData_->timestamp;
    for (auto& position : ballData_->positions)
    {

      // If the current NAO is the keeper, filter out ball candidates which
      // are farther away than a third of the length of the field.
      // This specifically avoids false positives which occur in the center circle.
      if (playerConfiguration_->playerNumber == 1)
      {
        const float keeper_threshold =
            fieldDimensions_->fieldLength / 2 - fieldDimensions_->fieldCenterCircleDiameter / 2;
        if (position.norm() < keeper_threshold)
        {
          update(position);
        }
      }
      else
      {
        if (position.norm() < 6.f)
        {
          update(position);
        }
      }
    }
  }
  selectBestMode();
  if (bestMode_ == ballModes_.end())
  {
    // No ball available.
    ballState_->position = Vector2f(0, 0);
    ballState_->velocity = Vector2f(0, 0);
    ballState_->destination = Vector2f(0, 0);
    ballState_->age = 1337;
    ballState_->found = false;
    ballState_->moved = false;
    ballState_->confident = false;
    ballState_->timeWhenBallLost = timeWhenBallLost_;
    ballState_->timeWhenLastSeen = 0;
  }
  else
  {
    if (bestMode_->resting)
    {
      ballState_->position = bestMode_->restingEquivalent.x;
      ballState_->velocity = {0.f, 0.f};
    }
    else
    {
      ballState_->position = bestMode_->movingEquivalent.x;
      ballState_->velocity = bestMode_->movingEquivalent.dx;
    }
    ballState_->destination = predictBallDestination(*bestMode_);
    ballState_->found = true;
    ballState_->moved = !bestMode_->resting;
    ballState_->age = cycleInfo_->getTimeDiff(bestMode_->lastUpdate);
    ballState_->confident = bestMode_->measurements >= confidentMeasurementThreshold_();
    ballState_->timeWhenLastSeen = bestMode_->lastUpdate;
    timeWhenBallLost_ = cycleInfo_->startTime;
  }
  sendDebug();
}

Vector2f BallFilter::predictBallDestination(const BallMode& ballMode) const
{
  if (ballMode.resting)
  {
    return ballMode.restingEquivalent.x;
  }

  const float v0Squared = ballMode.movingEquivalent.dx.squaredNorm();
  const float rollingDistance = 0.5f * v0Squared / frictionDeceleration_;
  Vector2f rollingDirection = ballMode.movingEquivalent.dx;
  rollingDirection.normalize();

  return ballMode.movingEquivalent.x + rollingDirection * rollingDistance;
}

void BallFilter::predict()
{
  // Remove old modes.
  ballModes_.erase(std::remove_if(ballModes_.begin(), ballModes_.end(),
                                  [this](const BallMode& mode) {
                                    // The more measurements there are for a mode, the longer it is
                                    // allowed to stay in the filter.
                                    return cycleInfo_->getTimeDiff(mode.lastUpdate) >
                                           (mode.measurements < 10
                                                ? static_cast<float>(mode.measurements) / 2.f
                                                : 5.f);
                                  }),
                   ballModes_.end());

  const Pose& odometry = odometryOffset_->odometryOffset;
  const Rotation2Df r(-odometry.orientation);
  const Pose invOdometry = odometry.inverse();
  for (auto& mode : ballModes_)
  {
    // predict the moving ball hypothesis
    mode.movingEquivalent.x = invOdometry * mode.movingEquivalent.x;
    mode.movingEquivalent.dx = r * mode.movingEquivalent.dx;
    // predict the resting ball hypothesis
    mode.restingEquivalent.x = invOdometry * mode.restingEquivalent.x;
  }

  float dt = cycleInfo_->getTimeDiff(lastPrediction_);
  lastPrediction_ = cycleInfo_->startTime;

  for (auto& mode : ballModes_)
  {
    // basic friction model:
    // m*dv = m*ddx = F, where F is the friction force and m is the mass of the ball
    // F = m * mu, mu = 0.1 (friction parameter to be determined by experiments)
    // -> dv = F / m * dt = mu * dt
    float vel = mode.movingEquivalent.dx.norm();
    if (vel <= numOfRestingDeccelerationSteps_() * frictionDeceleration_ * dt)
    {
      mode.movingEquivalent.dx = {0, 0};
      if (!mode.resting && mode.measurements > 30)
      {
        mode.resting = true;
        // Reset the resting ball equivalent to the place the new reseting position is assumed to be
        mode.restingEquivalent.x = mode.movingEquivalent.x;
      }
    }
    else
    {
      mode.movingEquivalent.dx -= mode.movingEquivalent.dx / vel * frictionDeceleration_ * dt;
    }
    mode.movingEquivalent.x += mode.movingEquivalent.dx * dt;

    // This is the Kalman filter equation P := F * P * F' + Q.
    // for the moving ball hypothesis
    mode.movingEquivalent.covX +=
        ((mode.movingEquivalent.covDxX + mode.movingEquivalent.covDxX.transpose()) +
         mode.movingEquivalent.covDx * dt) *
            dt +
        movingProcessCovX_();
    mode.movingEquivalent.covDxX += mode.movingEquivalent.covDx * dt + movingProcessCovDxX_();
    mode.movingEquivalent.covDx += movingProcessCovDx_();
    // for the resting ball hypothesis
    mode.restingEquivalent.covX += restingProcessCovX_();
  }
}

void BallFilter::updateMovingEquivalent(MovingEquivalent& movingEquivalent,
                                        const Vector2f& measurementMean,
                                        const Matrix2f& measurementCov)
{
  // compute the new association error
  const float newError = (measurementMean - movingEquivalent.x).norm();
  movingEquivalent.error = movingEquivalent.error * movingErrorLowPassAlpha_() +
                           newError * (1 - movingErrorLowPassAlpha_());

  // The comments show which code corresponds to which Kalman filter equation.
  // Be aware that x in the comments denotes the complete state, i.e. the vector [ x y dx dy ]'
  // y := z - H * x (in our case, H draws the first two components of the state vector)
  Vector2f residual = measurementMean - movingEquivalent.x;
  // S := H * P * H' + R (in our case, H * P * H' gets the covariance of the position)
  // Since only the inverse of S is needed, it is precomputed.
  Matrix2f residualCovInv = (movingEquivalent.covX + measurementCov).inverse();
  // K := P * H' * inv(S) is not computed explicitly.
  // x := x + K * y (splitted into parts for position and velocity)
  movingEquivalent.x += movingEquivalent.covX * residualCovInv * residual;
  movingEquivalent.dx += movingEquivalent.covDxX * residualCovInv * residual;
  // P := (I - K * H) * P
  // The order of these computations is chosen in a way that each covariance matrix is based on the
  // covariance matrices before the update. Every other order would break this.
  movingEquivalent.covDx -=
      movingEquivalent.covDxX * residualCovInv * movingEquivalent.covDxX.transpose();
  movingEquivalent.covDxX -= movingEquivalent.covDxX * residualCovInv * movingEquivalent.covX;
  movingEquivalent.covX -= movingEquivalent.covX * residualCovInv * movingEquivalent.covX;
}

void BallFilter::updateRestingEquivalent(RestingEquivalent& restingEquivalent,
                                         const Vector2f& measurementMean,
                                         const Matrix2f& measurementCov)
{
  const float newError = (measurementMean - restingEquivalent.x).norm();
  restingEquivalent.error = restingEquivalent.error * restingErrorLowPassAlpha_() +
                            newError * (1 - restingErrorLowPassAlpha_());

  // The comments show which code corresponds to which Kalman filter equation.
  // Be aware that x in the comments denotes the complete state, i.e. the vector [ x y ]'
  // y := z - H * x (in our case, H is the identity)
  Vector2f residual = measurementMean - restingEquivalent.x;
  // S := H * P * H' + R (in our case, H * P * H' is just P)
  // Since only the inverse of S is needed, it is precomputed.
  Matrix2f residualCovInv = (restingEquivalent.covX + measurementCov).inverse();
  // K := P * H' * inv(S) is not computed explicitly.
  // x := x + K * y
  restingEquivalent.x += restingEquivalent.covX * residualCovInv * residual;
  // P := (I - K * H) * P
  restingEquivalent.covX -= restingEquivalent.covX * residualCovInv * restingEquivalent.covX;
}

void BallFilter::update(const Vector2f& measurementMean)
{
  // estimate the covariance of the measurementMean from the projection uncertainty
  const auto measurementCov = projectionMeasurementModel_.computePointCovFromPositionFeature(
      measurementMean, cameraMatrix_->camera2ground);

  std::list<BallMode>::iterator nearestMode = ballModes_.end();
  // cope for the measurement uncertainty for association. This is a heuristic approach with out
  // deep mathematical meening.
  const float uncertaintyRadius =
      std::pow(std::max(measurementCov(0, 0), measurementCov(1)), 1.f / 8.f);
  float nearestDistance = uncertaintyRadius;
  // Find the nearest mode that is nearer than 1m to the measurementMean.
  for (auto mode = ballModes_.begin(); mode != ballModes_.end(); mode++)
  {
    float distance = (measurementMean - mode->movingEquivalent.x).norm();
    if (distance < nearestDistance)
    {
      nearestMode = mode;
      nearestDistance = distance;
    }
  }
  // If such a mode exists, combine prediction and measurementMean.
  if (nearestMode != ballModes_.end())
  {
    // update the moving part of the equivalent
    updateMovingEquivalent(nearestMode->movingEquivalent, measurementMean, measurementCov);
    // update the resting part of the equivalent
    updateRestingEquivalent(nearestMode->restingEquivalent, measurementMean, measurementCov);
    // if a ball is significantly moving, change the resting state
    if (nearestMode->restingEquivalent.error >
            relativeMovingThreshold_() * nearestMode->movingEquivalent.error &&
        nearestMode->restingEquivalent.error > maxRestingError_())
    {
      nearestMode->resting = false;
    }
    nearestMode->measurements++;
    nearestMode->lastUpdate = ballData_->timestamp;
  }
  else
  {
    // Create new mode.
    BallMode m;
    m.movingEquivalent.x = measurementMean;
    // Assume an initial velocity of 0.
    m.movingEquivalent.dx = Vector2f::Zero();
    m.movingEquivalent.covX = measurementBaseVariance_().asDiagonal();
    m.movingEquivalent.covDxX = Matrix2f::Identity();
    m.movingEquivalent.covDx = measurementBaseVariance_().asDiagonal();

    m.restingEquivalent.x = measurementMean;
    m.restingEquivalent.covX = Matrix2f::Identity();
    m.measurements = 1;
    m.lastUpdate = ballData_->timestamp;
    ballModes_.push_back(m);
  }
}

void BallFilter::selectBestMode()
{
  float bestScore = std::numeric_limits<float>::max();
  bestMode_ = ballModes_.end();
  for (auto mode = ballModes_.begin(); mode != ballModes_.end(); mode++)
  {
    if (mode->measurements < ballModes_.size())
    {
      continue;
    }
    float movingScore = mode->movingEquivalent.covX(0, 0) + mode->movingEquivalent.covX(1, 1);
    float restingScore = mode->restingEquivalent.covX(0, 0) + mode->restingEquivalent.covX(1, 1);
    float score = std::min(movingScore, restingScore);
    if (score < bestScore)
    {
      bestScore = score;
      bestMode_ = mode;
    }
  }
}

void BallFilter::sendDebug() const
{
  // debug data for the resting moving classification
  debug().update(mount_ + ".restingError", bestMode_->restingEquivalent.error);
  debug().update(mount_ + ".movingError", bestMode_->movingEquivalent.error);
  debug().update(mount_ + ".classification", (int)(bestMode_->resting));
  debug().update(mount_ + ".diff",
                 bestMode_->movingEquivalent.error - bestMode_->restingEquivalent.error);
  debug().update(mount_ + ".ratio",
                 bestMode_->restingEquivalent.error / bestMode_->movingEquivalent.error);

  // the final estiamte
  debug().update(mount_ + ".ballState", *ballState_);
  debug().update(mount_ + ".position", ballState_->position);
}
