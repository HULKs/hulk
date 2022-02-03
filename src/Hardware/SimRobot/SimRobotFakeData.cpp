#include "Hardware/SimRobot/SimRobotFakeData.hpp"
#include <iostream>

void SimRobotFakeData::setFakeRobotPose(const Pose& fakeData)
{
  std::lock_guard<std::mutex> l(fakeDataMutex_);
  fakeRobotPose_ = fakeData;
  fakeRobotPoseIsAvailable_ = true;
}

bool SimRobotFakeData::readFakeRobotPose(Pose& fakeData)
{
  std::lock_guard<std::mutex> l(fakeDataMutex_);
  fakeData = fakeRobotPose_;
  return fakeRobotPoseIsAvailable_;
}

void SimRobotFakeData::setFakeBallPosition(const Vector2f& fakeData)
{
  std::lock_guard<std::mutex> l(fakeDataMutex_);
  fakeBallPosition_ = fakeData;
  fakeBallIsAvailable_ = true;
}

bool SimRobotFakeData::readFakeBallPosition(Vector2f& fakeData)
{
  std::lock_guard<std::mutex> l(fakeDataMutex_);
  fakeData = fakeBallPosition_;
  return fakeBallIsAvailable_;
}

void SimRobotFakeData::setFakeRobotPositions(const VecVector2f& fakeData)
{
  std::lock_guard<std::mutex> l(fakeDataMutex_);
  fakeRobotPositions_ = fakeData;
  fakeRobotPositionsAreAvailable_ = true;
}

bool SimRobotFakeData::readFakeRobotPositions(VecVector2f& fakeData)
{
  std::lock_guard<std::mutex> l(fakeDataMutex_);
  fakeData = fakeRobotPositions_;
  return fakeRobotPositionsAreAvailable_;
}
