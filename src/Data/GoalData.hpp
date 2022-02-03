#pragma once

#include <vector>

#include "Framework/DataType.hpp"
#include "Hardware/Clock.hpp"
#include "Tools/Math/Eigen.hpp"

class GoalData : public DataType<GoalData>
{
public:
  /// the name of this DataType
  DataTypeName name__{"GoalData"};
  /// the positions of detected goal postts
  VecVector2f posts;
  /// the timestamp of the image in which they were seen
  Clock::time_point timestamp;
  /// whether the goal posts are valid
  bool valid = false;
  /**
   * @brief reset sets the goal to a defined state
   */
  void reset() override
  {
    valid = false;
    posts.clear();
  }

  void toValue(Uni::Value& value) const override
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["posts"] << posts;
    value["timestamp"] << timestamp;
    value["valid"] << valid;
  }

  void fromValue(const Uni::Value& value) override
  {
    value["posts"] >> posts;
    value["timestamp"] >> timestamp;
    value["valid"] >> valid;
  }
};
