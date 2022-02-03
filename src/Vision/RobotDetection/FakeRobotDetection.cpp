#include "Vision/RobotDetection/FakeRobotDetection.hpp"
#include "Tools/Math/Eigen.hpp"
#include "Tools/Math/Pose.hpp"

FakeRobotDetection::FakeRobotDetection(const ModuleManagerInterface& manager)
  : Module(manager)
  , fakeImageData_(*this)
  , cycleInfo_(*this)
  , fakeRobotData_(*this)
{
}

void FakeRobotDetection::cycle()
{
  VecVector2f relativeFakeRobotPositions;
  // get the data from the interface
  auto& fakeDataInterface = robotInterface().getFakeData();
  bool fakeRobotsAvailable = fakeDataInterface.readFakeRobotPositions(relativeFakeRobotPositions);
  // construct the robot pose to tranform the absolute fake data to relative coordinates
  if (fakeRobotsAvailable)
  {
    fakeRobotData_->positions = relativeFakeRobotPositions;
    fakeRobotData_->timestamp = cycleInfo_->startTime;
  }
}
