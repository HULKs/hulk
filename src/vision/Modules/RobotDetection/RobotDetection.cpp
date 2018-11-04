#include "RobotDetection.hpp"

RobotDetection::RobotDetection(const ModuleManagerInterface& manager)
  : Module(manager)
  , cycleInfo_(*this)
  , robotData_(*this)
{
}

void RobotDetection::cycle()
{
}
