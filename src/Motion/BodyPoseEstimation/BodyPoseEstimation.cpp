#include "Motion/BodyPoseEstimation/BodyPoseEstimation.hpp"


BodyPoseEstimation::BodyPoseEstimation(const ModuleManagerInterface& manager)
  : Module{manager}
  , actionCommand_{*this}
  , cycleInfo_{*this}
  , standUpResult_{*this}
  , imuSensorData_{*this}
  , fsrSensorData_{*this}
  , motionActivation_{*this}
  , bodyPose_{*this}
  , uprightUpThreshX_{*this, "uprightUpThreshX", [] {}}
  , uprightUpThreshY_{*this, "uprightUpThreshY", [] {}}
  , uprightLoThreshZ_{*this, "uprightLoThreshZ", [] {}}
  , minFsrPressure_{*this, "minFsrPressure", [] {}}
  , maxFsrPressure_{*this, "maxFsrPressure", [] {}}
  , outerFsrWeight_{*this, "outerFsrWeight", [] {}}
  , innerFsrWeight_{*this, "innerFsrWeight", [] {}}
  , weightThreshold_{*this, "weightThreshold", [] {}}
  , classifyHighByGyro_{*this, "classifyHighByGyro", [] {}}
  , movingGyroNormThreshold_{*this, "movingGyroNormThreshold", [] {}}
  , xMin_{*this, "xMin", [] {}}
  , xMax_{*this, "xMax", [] {}}
  , yMin_{*this, "yMin", [] {}}
  , yMax_{*this, "yMax", [] {}}
  , xdMin_{*this, "xdMin", [] {}}
  , xdMax_{*this, "xdMax", [] {}}
  , ydMin_{*this, "ydMin", [] {}}
  , ydMax_{*this, "ydMax", [] {}}
  , maxGyroNormNotWonky_{*this, "maxGyroNormNotWonky", [] {}}
{
  weightBuffer_.fill(0.f);

  weights_[FSRs::L_FRONT_LEFT] = weights_[FSRs::L_REAR_LEFT] = outerFsrWeight_();
  weights_[FSRs::L_FRONT_RIGHT] = weights_[FSRs::L_REAR_RIGHT] = innerFsrWeight_();
  weights_[FSRs::R_FRONT_LEFT] = weights_[FSRs::R_REAR_LEFT] = -innerFsrWeight_();
  weights_[FSRs::R_FRONT_RIGHT] = weights_[FSRs::R_REAR_RIGHT] = -outerFsrWeight_();

  for (float& pressure : highestPressure_)
  {
    pressure = minFsrPressure_();
  }
}

void BodyPoseEstimation::cycle()
{
  detectFalling();
  detectWonky();
  determineApproxUpright();
  determineFootContact();
  determineSupportFoot();
}

void BodyPoseEstimation::detectFalling()
{
  // If StandUp says it stood up successfully, we believe that the robot is not fallen anymore.
  if (standUpResult_->finishedSuccessfully)
  {
    fallen_ = false;
  }

  // filter sensor readings
  const float alpha = 0.3f;
  angleAccumulator_ = imuSensorData_->angle * alpha + angleAccumulator_ * (1 - alpha);
  gyroAccumulator_ = imuSensorData_->gyroscope * alpha + gyroAccumulator_ * (1 - alpha);

  // for each direction, check if angle and angular velocity exceed their respective limit
  if (!fallen_)
  {
    if (angleAccumulator_.x() < xMin_() && gyroAccumulator_.x() < xdMin_())
    {
      fallDirection_ = BodyPose::FallDirection::LEFT;
    }
    else if (angleAccumulator_.x() > xMax_() && gyroAccumulator_.x() > xdMax_())
    {
      fallDirection_ = BodyPose::FallDirection::RIGHT;
    }
    else if (angleAccumulator_.y() < yMin_() && gyroAccumulator_.y() < ydMin_())
    {
      fallDirection_ = BodyPose::FallDirection::BACK;
    }
    else if (angleAccumulator_.y() > yMax_() && gyroAccumulator_.y() > ydMax_())
    {
      fallDirection_ = BodyPose::FallDirection::FRONT;
    }
    else
    {
      fallDirection_ = BodyPose::FallDirection::NOT_FALLING;
    }
  }
  // If the robot was not previously fallen but is falling now, it is fallen.
  if (fallDirection_ != BodyPose::FallDirection::NOT_FALLING && !fallen_)
  {
    fallen_ = true;
    timeWhenFallen_ = cycleInfo_->startTime;
  }
  // Expose the fallen state to other modules.
  bodyPose_->fallen = fallen_;
  bodyPose_->fallDirection = fallDirection_;
  bodyPose_->timeWhenFallen = timeWhenFallen_;
}

void BodyPoseEstimation::detectWonky()
{
  bodyPose_->wonky = imuSensorData_->gyroscope.norm() > maxGyroNormNotWonky_();
}

void BodyPoseEstimation::determineApproxUpright()
{
  bodyPose_->upright = std::abs(imuSensorData_->accelerometer.x()) <= uprightUpThreshX_() &&
                       std::abs(imuSensorData_->accelerometer.y()) <= uprightUpThreshY_() &&
                       std::abs(imuSensorData_->accelerometer.z()) >= uprightLoThreshZ_();
}

void BodyPoseEstimation::determineFootContact()
{
  using MotionType = ActionCommand::Body::MotionType;
  const auto& bodyMotionState{motionActivation_->activeMotion};
  if ((bodyMotionState == MotionType::STAND || bodyMotionState == MotionType::PENALIZED) &&
      !(lastBodyMotionType_ == MotionType::STAND || lastBodyMotionType_ == MotionType::PENALIZED))
  {
    // reset filter as soon as nao is standing or penalized
    filteredGyroNorm_ = 0.f;
  }
  const float lowPassGain = 0.2f;
  filteredGyroNorm_ =
      (1.f - lowPassGain) * filteredGyroNorm_ + lowPassGain * imuSensorData_->gyroscope.norm();
  lastBodyMotionType_ = bodyMotionState;
  if (classifyHighByGyro_())
  {
    if (bodyMotionState == MotionType::STAND || bodyMotionState == MotionType::PENALIZED)
    {
      // when standing or penalized the filtered gyro norm is expected to be low
      if (filteredGyroNorm_ > movingGyroNormThreshold_())
      {
        // the gyro norm is suspiciously high - assumed to be picked up manually
        bodyPose_->timeOfLastFootContact = timeOfLastFootContact_;
        bodyPose_->footContact = false;
        return;
      }
      // if the gyro norm doesn't consider the robot to be high, the usual classification by FSRs is
      // further used
    }
  }
  float totalWeight = fsrSensorData_->totalLeft + fsrSensorData_->totalRight;
  // The average over the last few FSR sensor values is computed.
  weightBufferSum_ -= weightBuffer_[weightBufferPosition_];
  weightBuffer_[weightBufferPosition_++] = totalWeight;
  weightBufferPosition_ %= weightBufferSize__;
  weightBufferSum_ += totalWeight;
  // If the average weight on the FSRs exceeds a threshold, the robot is assumed to touch something
  // with at least one foot.
  bodyPose_->footContact = (weightBufferSum_ / weightBufferSize__) > weightThreshold_();
  debug().update(mount_ + ".fsr_both_feed_sum", (weightBufferSum_ / weightBufferSize__));

  if (bodyPose_->footContact)
  {
    timeOfLastFootContact_ = cycleInfo_->startTime;
  }
  bodyPose_->timeOfLastFootContact = timeOfLastFootContact_;
}

void BodyPoseEstimation::determineSupportFoot()
{
  float totalPressure = 0.f;
  float weightedSum = 0.f;

  FSRsArray<float> fsrReadings = {
      {fsrSensorData_->leftFoot.frontLeft, fsrSensorData_->leftFoot.frontRight,
       fsrSensorData_->leftFoot.rearLeft, fsrSensorData_->leftFoot.rearRight,
       fsrSensorData_->rightFoot.frontLeft, fsrSensorData_->rightFoot.frontRight,
       fsrSensorData_->rightFoot.rearLeft, fsrSensorData_->rightFoot.rearRight}};

  for (std::size_t i = 0; i < static_cast<std::size_t>(FSRs::MAX); i++)
  {
    const auto fsr = static_cast<FSRs>(i);
    float pressure = std::min(maxFsrPressure_(), fsrReadings[fsr]);
    highestPressure_[fsr] = std::max(highestPressure_[fsr], pressure);
    pressure /= highestPressure_[fsr];
    totalPressure += pressure;
    weightedSum += weights_[fsr] * pressure;
  }

  if (std::abs(totalPressure) > 0.f)
  {
    bodyPose_->supportSide = weightedSum / totalPressure;
    bodyPose_->supportChanged = lastSupportSide_ * bodyPose_->supportSide < 0.f;
  }
  else
  {
    bodyPose_->supportSide = 0.f;
    bodyPose_->supportChanged = false;
  }

  lastSupportSide_ = bodyPose_->supportSide;
}

void BodyPoseEstimation::sendAngleExtremes()
{
  const Vector2f& temp = imuSensorData_->angle;

  if (temp.x() < tempXmin_)
  {
    tempXmin_ = temp.x();
    debug().update(mount_ + ".x_min", tempXmin_);
  }
  else if (temp.x() > tempXmax_)
  {
    tempXmax_ = temp.x();
    debug().update(mount_ + ".x_max", tempXmax_);
  }

  if (temp.y() < tempYmin_)
  {
    tempYmin_ = temp.y();
    debug().update(mount_ + ".y_min", tempYmin_);
  }
  else if (temp.y() > tempYmax_)
  {
    tempYmax_ = temp.y();
    debug().update(mount_ + ".y_max", tempYmax_);
  }
}
