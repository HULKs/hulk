#include <cassert>
#include <fstream>
#include <iostream>

#include "Modules/NaoProvider.h"
#include "print.h"

#include "Poses.h"


const char* Poses::poseFiles[Poses::POSE_MAX] = {
    "poses/AL_Init.pose",  "poses/ArmBackStage1.pose", "poses/ArmBackStage2.pose",
    "poses/Home.pose",     "poses/Penalized.pose",     "poses/Ready.pose",
    "poses/TakeAway.pose", "poses/Transport.pose"};

std::vector<float> Poses::poses[POSE_MAX];

const std::vector<float>& Poses::getPose(const EnumPose index)
{
  assert(index < POSE_MAX);
  assert(poses[index].size() == JOINTS::JOINTS_MAX);
  return poses[index];
}

bool Poses::init(const std::string& fileRoot)
{
  static bool initialized = false;
  if (initialized)
  {
    return true;
  }
  for (unsigned int i = 0; i < POSE_MAX; i++)
  {
    std::string path(fileRoot + poseFiles[i]);
    std::ifstream file(path, std::ios::in);

    if (!file)
    {
      print("File: " + path + "could not be opened", LogLevel::ERROR);
      return false;
    }

    std::vector<float>& poseVector = poses[i];
    poseVector.resize(JOINTS::JOINTS_MAX);

    for (unsigned int j = 0; j < JOINTS::JOINTS_MAX; j++)
    {
      file >> poseVector[j];
    }
  }
  initialized = true;
  return true;
}
