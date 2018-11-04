#include <iostream>
#include "NaoFakeData.hpp"

void NaoFakeData::waitForFakeData() {}

bool NaoFakeData::readFakeRobotPose(Pose& /*fakeData*/)
{
  return false;
}

bool NaoFakeData::readFakeBallPosition(Vector2f& /*fakeData*/)
{
  std::lock_guard<std::mutex> l(fakeDataMutex_);
  return false;
}

bool NaoFakeData::readFakeRobotPositions(VecVector2f& /*fakeData*/)
{
  std::lock_guard<std::mutex> l(fakeDataMutex_);
  return false;
}
