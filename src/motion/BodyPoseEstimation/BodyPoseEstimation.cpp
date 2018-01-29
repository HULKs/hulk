#include "BodyPoseEstimation.hpp"

BodyPoseEstimation::BodyPoseEstimation(const ModuleManagerInterface& manager)
  : Module(manager, "BodyPoseEstimation")
  , weightThreshold_(*this, "weightThreshold", [] {})
  , classifyHighByGyro_(*this, "classifyHighByGyro", [] {})
  , movingGyroNormThreshold_(*this, "movingGyroNormThreshold", [] {})
  , xMin_(*this, "xMin", [] {})
  , xMax_(*this, "xMax", [] {})
  , yMin_(*this, "yMin", [] {})
  , yMax_(*this, "yMax", [] {})
  , xdMin_(*this, "xdMin", [] {})
  , xdMax_(*this, "xdMax", [] {})
  , ydMin_(*this, "ydMin", [] {})
  , ydMax_(*this, "ydMax", [] {})
  , cycleInfo_(*this)
  , standUpResult_(*this)
  , imuSensorData_(*this)
  , fsrSensorData_(*this)
  , motionState_(*this)
  , bodyPose_(*this)
  , fallen_(false)
  , filteredGyroNorm_(0.f)
  , lastBodyMotionState_(BodyMotion::DEAD)
  , timeWhenFallen_()
  , timeOfLastFootContact_()
  , weightBufferPosition_(0)
  , weightBufferSum_(0)
  , angleAccumulator_(0, 0, 0)
  , gyroAccumulator_(0, 0, 0)
{
  weightBuffer_.fill(0);
}

void BodyPoseEstimation::cycle()
{
  detectFalling();
  determineFootContact();
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
  if (angleAccumulator_.x() < xMin_() && gyroAccumulator_.x() < xdMin_())
  {
    bodyPose_->fallDirection = FallDirection::LEFT;
  }
  else if (angleAccumulator_.x() > xMax_() && gyroAccumulator_.x() > xdMax_())
  {
    bodyPose_->fallDirection = FallDirection::RIGHT;
  }
  else if (angleAccumulator_.y() < yMin_() && gyroAccumulator_.y() < ydMin_())
  {
    bodyPose_->fallDirection = FallDirection::BACK;
  }
  else if (angleAccumulator_.y() > yMax_() && gyroAccumulator_.y() > ydMax_())
  {
    bodyPose_->fallDirection = FallDirection::FRONT;
  }
  else
  {
    bodyPose_->fallDirection = FallDirection::NOT_FALLING;
  }
  // If the robot was not previously fallen but is falling now, it is fallen.
  if (bodyPose_->fallDirection != FallDirection::NOT_FALLING && !fallen_)
  {
    fallen_ = true;
    timeWhenFallen_ = cycleInfo_->startTime;
  }
  // Expose the fallen state to other modules.
  bodyPose_->fallen = fallen_;
  bodyPose_->timeWhenFallen = timeWhenFallen_;
}

void BodyPoseEstimation::determineFootContact()
{
  const auto& bodyMotionState = motionState_->bodyMotion;
  if ((bodyMotionState == BodyMotion::STAND || bodyMotionState == BodyMotion::PENALIZED) &&
      !(lastBodyMotionState_ == BodyMotion::STAND || lastBodyMotionState_ == BodyMotion::PENALIZED))
  {
    // reset filter as soon as nao is standing or penalized
    filteredGyroNorm_ = 0.f;
  }
  const float lowPassGain = 0.2f;
  filteredGyroNorm_ = (1.f - lowPassGain) * filteredGyroNorm_ + lowPassGain * imuSensorData_->gyroscope.norm();
  lastBodyMotionState_ = bodyMotionState;
  if (classifyHighByGyro_())
  {
    if (bodyMotionState == BodyMotion::STAND || bodyMotionState == BodyMotion::PENALIZED)
    {
      // when standing or penalized the filtered gyro norm is expected to be low
      if (filteredGyroNorm_ > movingGyroNormThreshold_())
      {
        // the gyro norm is suspiciously high - assumed to be picked up manually
        bodyPose_->timeOfLastFootContact = timeOfLastFootContact_;
        bodyPose_->footContact = false;
        return;
      }
      // if the gyro norm doesn't consider the robot to be high, the usual classification by FSRs is further used
    }
  }
  float totalWeight = fsrSensorData_->left.totalWeight + fsrSensorData_->right.totalWeight;
  // The average over the last few FSR sensor values is computed.
  weightBufferSum_ -= weightBuffer_[weightBufferPosition_];
  weightBuffer_[weightBufferPosition_++] = totalWeight;
  weightBufferPosition_ %= weightBufferSize_;
  weightBufferSum_ += totalWeight;
  // If the average weight on the FSRs exceeds a threshold, the robot is assumed to touch something with at least one foot.
  bodyPose_->footContact = (weightBufferSum_ / weightBufferSize_) > weightThreshold_();
  debug().update(mount_ + ".fsr_both_feed_sum", (weightBufferSum_ / weightBufferSize_));

  if (bodyPose_->footContact)
  {
    timeOfLastFootContact_ = cycleInfo_->startTime;
  }
  bodyPose_->timeOfLastFootContact = timeOfLastFootContact_;
}

void BodyPoseEstimation::sendAngleExtremes()
{
  const Vector3f& temp = imuSensorData_->angle;

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
