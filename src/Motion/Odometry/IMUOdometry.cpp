#include "Motion/Odometry/IMUOdometry.hpp"
#include "Data/ActionCommand.hpp"
#include "Tools/Math/KinematicMatrix.hpp"
#include <type_traits>

IMUOdometry::IMUOdometry(const ModuleManagerInterface& manager)
  : Module(manager)
  , sensorFusion_(*this)
  , robotKinematics_(*this)
  , cycleInfo_(*this)
  , imuSensorData_(*this)
  , bodyPose_(*this)
  , motionState_(*this)
  , bodyRotationData_(*this)
  , odometryData_(*this)
{
}

void IMUOdometry::cycle()
{
  static_assert(std::is_same_v<Clock::duration::period, std::chrono::seconds::period>);
  sensorFusion_.update(imuSensorData_->gyroscope, imuSensorData_->accelerometer,
                       cycleInfo_->cycleTime.count());

  Vector3f rpy = sensorFusion_.getOrientation();
  debug().update(mount_ + ".Orientation", rpy);

  accumulatedOdometry_ *= Pose{robotKinematics_->lastGround2currentGround};
  accumulatedOdometry_.angle() = rpy.z();

  odometryData_->accumulatedOdometry = accumulatedOdometry_;

  // produce bodyRotionData_
  bodyRotationData_->rollPitchYaw = rpy;
  bodyRotationData_->bodyTilt2ground = sensorFusion_.getBodyTilt();

  detectOrientationDrift();
}

void IMUOdometry::detectOrientationDrift()
{
  const auto& bodyMotionState = motionState_->bodyMotion;

  // current acceleration without gravity
  const float acceleration = abs(imuSensorData_->accelerometer.norm() - 9.81f);

  // drift detection can only be done when we are not moving or moved by e.g. a referee
  if ((bodyMotionState != ActionCommand::Body::MotionType::STAND &&
       bodyMotionState != ActionCommand::Body::MotionType::PENALIZED) ||
      !bodyPose_->footContact || acceleration > .6f)
  {
    return;
  }

  const float orientation = odometryData_->accumulatedOdometry.angle();
  const float orientationDelta = lastOrientation_ - orientation;

  deltaOrientationAverage_.put(orientationDelta);

  // only check for gyro drift every 50 cycles. It is intended to not check this in the first
  // cycle (to let the buffer fill up a bit)
  if (++cycleSinceLastDriftCheck_ % 50 == 0)
  {
    Log<M_MOTION>(LogLevel::DEBUG) << "Average is " << deltaOrientationAverage_.getAverage()
                                   << ", Range was " << deltaOrientationAverage_.getRange();

    if (abs(deltaOrientationAverage_.getAverage()) > .00005f &&
        deltaOrientationAverage_.getRange() < .001f)
    {
      // Measured drift
      Log<M_MOTION>(LogLevel::WARNING)
          << "Drifted, average is " << deltaOrientationAverage_.getAverage() << ", range was "
          << deltaOrientationAverage_.getRange();
      debug().playAudio("orientation_drift_detected", AudioSounds::DRIFT);
    }
    cycleSinceLastDriftCheck_ = 0;
  }

  lastOrientation_ = orientation;
}
