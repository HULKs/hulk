#include "Hardware/Replay/ReplayFakeData.hpp"
#include "Data/HeadMatrixBuffer.hpp"
#include <iostream>

bool ReplayFakeData::getFakeDataInternal(const std::type_index& id, DataTypeBase& data)
{
  if (id == typeid(ReplayFrame))
  {
    Uni::Value aux;
    currentFrame.toValue(aux);
    data.fromValue(aux);
    return true;
  }
  if (id == typeid(HeadMatrixBuffer))
  {
    if (!currentFrame.headMatrixBuffer.valid)
    {
      return false;
    }
    Uni::Value aux;
    currentFrame.headMatrixBuffer.toValue(aux);
    data.fromValue(aux);
    return true;
  }
  if (id == typeid(ReplayConfigurations))
  {
    Uni::Value aux;
    aux << replayConfig;
    data.fromValue(aux);
    return true;
  }
  return false;
}

bool ReplayFakeData::readFakeRobotPose(Pose& /*fakeData*/)
{
  return false;
}

bool ReplayFakeData::readFakeBallPosition(Vector2f& /*fakeData*/)
{
  std::lock_guard<std::mutex> l(fakeDataMutex_);
  return false;
}

bool ReplayFakeData::readFakeRobotPositions(VecVector2f& /*fakeData*/)
{
  std::lock_guard<std::mutex> l(fakeDataMutex_);
  return false;
}
