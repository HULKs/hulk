#include "Brain/OdometryOffsetProvider/OdometryOffsetProvider.hpp"


OdometryOffsetProvider::OdometryOffsetProvider(const ModuleManagerInterface& manager)
  : Module(manager)
  , odometryData_(*this)
  , odometryOffset_(*this)
  , initialized_(false)
{
}

void OdometryOffsetProvider::cycle()
{
  if (!initialized_)
  {
    lastOdometry_ = odometryData_->accumulatedOdometry;
    initialized_ = true;
    return;
  }

  odometryOffset_->odometryOffset = lastOdometry_.inverse() * odometryData_->accumulatedOdometry;
  lastOdometry_ = odometryData_->accumulatedOdometry;
}
