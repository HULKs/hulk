#pragma once

#include <cassert>

#include "Framework/DataType.hpp"
#include "Hardware/Clock.hpp"
#include "Tools/Math/KinematicMatrix.hpp"


struct HeadMatrixWithTimestamp : public Uni::To, public Uni::From
{
  HeadMatrixWithTimestamp() = default;
  HeadMatrixWithTimestamp(KinematicMatrix head2torso, KinematicMatrix torso2ground,
                          const Clock::time_point& timestamp)
    : head2torso(std::move(head2torso))
    , torso2ground(std::move(torso2ground))
    , timestamp(timestamp)
  {
  }
  /// a matrix describing the transformation from the head coordinate system to the torso coordinate
  /// system
  KinematicMatrix head2torso;
  /// a matrix describing the transformation from the torso coordinate system to the robot
  /// coordinate system
  KinematicMatrix torso2ground;
  /// the time at which the joints for this matrix have been recorded
  Clock::time_point timestamp;

  void toValue(Uni::Value& value) const override
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["head2torso"] << head2torso;
    value["torso2ground"] << torso2ground;
    value["timestamp"] << timestamp;
  }

  void fromValue(const Uni::Value& value) override
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
  DataTypeName name__{"HeadMatrixBuffer"};
  /// the buffer of matrices
  std::vector<HeadMatrixWithTimestamp> buffer;
  /// whether the content is valid
  bool valid = true;
  /**
   * @brief getBestMatch returns the head matrix that was recorded closest to a given timestamp
   * Callers must ensure that the buffer is not empty.
   * @param timestamp the time for which the appropriate matrix should be found
   * @return the closest matrix/Clock::time_point pair for the given timestamp
   */
  const HeadMatrixWithTimestamp& getBestMatch(const Clock::time_point timestamp) const
  {
    assert(!buffer.empty());
    auto minIt = buffer.begin(); // To make sure that there is a valid entry.
    auto minDiff = Clock::duration::max();
    for (auto it = buffer.begin(); it != buffer.end(); it++)
    {
      // For each element in the buffer, the difference to the searched timestamp is computed.
      const auto diff = std::chrono::abs(timestamp - it->timestamp);
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
  void reset() override
  {
    buffer.clear();
    valid = false;
  }

  void toValue(Uni::Value& value) const override
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["buffer"] << buffer;
    value["valid"] << valid;
  }

  void fromValue(const Uni::Value& value) override
  {
    value["buffer"] >> buffer;
    value["valid"] >> valid;
  }
};
