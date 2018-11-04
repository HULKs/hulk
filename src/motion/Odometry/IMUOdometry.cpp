#include "Tools/Kinematics/KinematicMatrix.h"

#include "IMUOdometry.hpp"


IMUOdometry::IMUOdometry(const ModuleManagerInterface& manager)
  : Module(manager)
  , sensorFusion_(*this)
  , walkingEngineWalkOutput_(*this)
  , imuSensorData_(*this)
  , bodyRotationData_(*this)
  , odometryData_(*this)
  , accumulatedOdometry_()
{
}

void IMUOdometry::cycle()
{
  sensorFusion_.update(imuSensorData_->gyroscope, imuSensorData_->accelerometer);

  Vector3f rpy = sensorFusion_.getOrientation();
  debug().update(mount_ + ".Orientation", rpy);

  accumulatedOdometry_.orientation = rpy.z();
  accumulatedOdometry_.position = accumulatedOdometry_ * walkingEngineWalkOutput_->stepOffset.position;

  odometryData_->accumulatedOdometry = accumulatedOdometry_;

  // produce bodyRotionData_
  bodyRotationData_->rollPitchYaw = rpy;
  bodyRotationData_->bodyTilt2ground = sensorFusion_.getBodyTilt();
}
