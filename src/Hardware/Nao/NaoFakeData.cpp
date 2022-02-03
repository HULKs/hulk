#include "Hardware/Nao/NaoFakeData.hpp"
#include <iostream>

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
