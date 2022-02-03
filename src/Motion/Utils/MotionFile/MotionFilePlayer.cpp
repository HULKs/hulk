#include "Motion/Utils/MotionFile/MotionFilePlayer.hpp"
#include "Framework/Log/Log.hpp"
#include <cmath>


MotionFilePlayer::MotionFilePlayer(const CycleInfo& cycleInfo,
                                   const JointSensorData& jointSensorData)
  : cycleInfo_(cycleInfo)
  , jointSensorData_(jointSensorData)
  , startTime_()
{
}

bool MotionFilePlayer::loadFromFile(const std::string& filename)
{
  if (!MotionFile::loadFromFile(filename) || !verify())
  {
    return false;
  }
  precompile();
  return true;
}

int MotionFilePlayer::play()
{
  if (!isPlaying())
  {
    Log<M_MOTION>(LogLevel::DEBUG) << "MotionFile " << header.title << " actually playing...";
    startTime_ = cycleInfo_.startTime;

    for (unsigned int i = 0; i < header.joints.size(); i++)
    {
      startJointValues_.angles[static_cast<Joints>(i)] =
          jointSensorData_.angles[static_cast<Joints>(header.joints[i])];
    }
    if (!stiffnesses_.empty())
    {
      // Use the stiffnesses from the first frame because interpolation of all the stiffnesses is
      // not necessary.
      startJointValues_.stiffnesses = stiffnesses_[0];
    }
    // If stiffnesses is empty, startJointValues_.stiffnesses won't be used.
    return header.time;
  }

  Log<M_MOTION>(LogLevel::WARNING)
      << "MotionFile " << header.title << " already playing. Command discarded.";
  return 0;
}

void MotionFilePlayer::stop()
{
  if (!isPlaying())
  {
    Log<M_MOTION>(LogLevel::WARNING)
        << "MotionFile " << header.title << " is not playing. Can not stop it.";
    return;
  }
  Log<M_MOTION>(LogLevel::DEBUG) << "MotionFile " << header.title << " stopped.";
  startTime_ -= std::chrono::milliseconds(header.time);
}

MotionFilePlayer::JointValues MotionFilePlayer::cycle()
{
  JointValues result;
  unsigned int i = 0;
  float interpolationFactor = NAN;
  const auto motionTime{std::chrono::duration_cast<std::chrono::milliseconds>(
                            cycleInfo_.getAbsoluteTimeDifference(startTime_))
                            .count()};
  JointsArray<float> last{};
  JointsArray<float> next{};
  if (angles_.empty())
  {
    Log<M_MOTION>(LogLevel::ERROR) << "MotionFile " << header.title << " does not have angles";
    return result;
  }
  // Find the current keyframe.
  if (motionTime < angleTimes_[0])
  {
    interpolationFactor = static_cast<float>(motionTime) / angleTimes_[0];
    last = startJointValues_.angles;
    next = angles_[0];
  }
  else
  {
    // Find the first i for which the next pose time is in the future.
    for (i = 0; i < angleTimes_.size() - 1; i++)
    {
      if (angleTimes_[i + 1] > motionTime)
      {
        break;
      }
    }
    last = angles_[i];
    if (i == angleTimes_.size() - 1)
    {
      interpolationFactor = 0;
      // Set next anyway to avoid NaN or other unexpected things.
      next = angles_[i];
    }
    else
    {
      interpolationFactor =
          static_cast<float>(motionTime - angleTimes_[i]) / (angleTimes_[i + 1] - angleTimes_[i]);
      next = angles_[i + 1];
    }
  }
  for (i = 0; i < header.joints.size(); i++)
  {
    result.angles[static_cast<Joints>(header.joints[i])] =
        (1 - interpolationFactor) * last[static_cast<Joints>(i)] +
        interpolationFactor * next[static_cast<Joints>(i)];
  }
  // Provide a fallback for motion files that do not have stiffnesses.
  if (stiffnesses_.empty())
  {
    // Assume full stiffness any time.
    for (const auto i : header.joints)
    {
      result.stiffnesses[static_cast<Joints>(i)] = 1.0f;
    }
    Log<M_MOTION>(LogLevel::ERROR)
        << "Motion file " << header.title << " does not have stiffnesses";
  }
  else
  {
    // Find the current keyframe.
    if (motionTime < stiffnessTimes_[0])
    {
      interpolationFactor = static_cast<float>(motionTime) / stiffnessTimes_[0];
      last = startJointValues_.stiffnesses;
      next = stiffnesses_[0];
    }
    else
    {
      for (i = 0; i < stiffnessTimes_.size() - 1; i++)
      {
        if (stiffnessTimes_[i + 1] > motionTime)
        {
          break;
        }
      }
      last = stiffnesses_[i];
      if (i == stiffnessTimes_.size() - 1)
      {
        interpolationFactor = 0;
        next = stiffnesses_[i];
      }
      else
      {
        interpolationFactor = static_cast<float>(motionTime - stiffnessTimes_[i]) /
                              (stiffnessTimes_[i + 1] - stiffnessTimes_[i]);
        next = stiffnesses_[i + 1];
      }
    }
    for (i = 0; i < header.joints.size(); i++)
    {
      result.stiffnesses[static_cast<Joints>(header.joints[i])] =
          (1 - interpolationFactor) * last[static_cast<Joints>(i)] +
          interpolationFactor * next[static_cast<Joints>(i)];
    }
  }
  return result;
}

bool MotionFilePlayer::isPlaying() const
{
  // assuming initial startTime_ = 0 so the motion was never started
  return startTime_ != Clock::time_point{} &&
         cycleInfo_.getAbsoluteTimeDifference(startTime_) < std::chrono::milliseconds(header.time);
}

void MotionFilePlayer::precompile()
{
  int relTimeASum = 0;
  int relTimeSSum = 0;

  angles_.clear();
  angleTimes_.clear();
  stiffnesses_.clear();
  stiffnessTimes_.clear();

  angles_.reserve(position.size());
  angleTimes_.reserve(position.size());
  stiffnesses_.reserve(stiffness.size());
  stiffnessTimes_.reserve(stiffness.size());

  for (auto& pos : position)
  {
    relTimeASum += pos.time;
  }
  for (auto& stiff : stiffness)
  {
    relTimeSSum += stiff.time;
  }

  int time = 0;
  for (auto& pos : position)
  {
    time += (pos.time * header.time) / relTimeASum;

    angleTimes_.push_back(time);
    auto& angles = angles_.emplace_back();
    assert(pos.parameters.size() == angles.size());
    std::copy_n(pos.parameters.begin(), angles.size(), angles.begin());
  }

  time = 0;
  for (auto& stiff : stiffness)
  {
    time += (stiff.time * header.time) / relTimeSSum;

    stiffnessTimes_.push_back(time);
    auto& stiffnesses = stiffnesses_.emplace_back();
    assert(stiff.parameters.size() == stiffnesses.size());
    std::copy_n(stiff.parameters.begin(), stiffnesses.size(), stiffnesses.begin());
  }
}
