#pragma once

#include <cassert>

#include "Framework/DataType.hpp"
#include "Tools/Kinematics/KinematicMatrix.h"
#include "Tools/Time.hpp"


struct HeadMatrixWithTimestamp : public Uni::To, public Uni::From
{
  /// a matrix describing the transformation from the head coordinate system to the torso coordinate
  /// system
  KinematicMatrix head2torso;
  /// a matrix describing the transformation from the torso coordinate system to the robot
  /// coordinate system
  KinematicMatrix torso2ground;
  /// the time at which the joints for this matrix have been recorded
  TimePoint timestamp;

  virtual void toValue(Uni::Value& value) const
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["head2torso"] << head2torso;
    value["torso2ground"] << torso2ground;
    value["timestamp"] << timestamp;
  }

  virtual void fromValue(const Uni::Value& value)
  {
    value["head2torso"] >> head2torso;
    value["torso2ground"] >> torso2ground;
    value["timestamp"] >> timestamp;
  }
};

class HeadMatrixBuffer : public DataType<HeadMatrixBuffer>
{
public:
  /// the name of this DataType
  DataTypeName name = "HeadMatrixBuffer";
  /// the buffer of matrices
  std::vector<HeadMatrixWithTimestamp> buffer;
  /// whether the content is valid
  bool valid = true;
  /**
   * @brief getBestMatch returns the head matrix that was recorded closest to a given timestamp
   * Callers must ensure that the buffer is not empty!
   * @param timestamp the time for which the appropriate matrix should be found
   * @return the closest matrix/timepoint pair for the given timestamp
   */
  const HeadMatrixWithTimestamp& getBestMatch(const TimePoint timestamp) const
  {
    assert(!buffer.empty());
    auto minIt = buffer.begin();                       // To make sure that there is a valid entry.
    float minDiff = std::numeric_limits<float>::max(); // Those are seconds.
    for (auto it = buffer.begin(); it != buffer.end(); it++)
    {
      // For each element in the buffer, the difference to the searched timestamp is computed.
      const float diff = std::abs(getTimeDiff(timestamp, it->timestamp, TDT::SECS));
      if (diff < minDiff)
      {
        minIt = it;
        minDiff = diff;
      }
    }
    return *minIt;
  }
  /**
   * @brief reset clears the buffer
   */
  void reset()
  {
    buffer.clear();
    valid = false;
  }

  virtual void toValue(Uni::Value& value) const
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["buffer"] << buffer;
    value["valid"] << valid;
  }

  virtual void fromValue(const Uni::Value& value)
  {
    value["buffer"] >> buffer;
    value["valid"] >> valid;
  }
};
