#pragma once

#include <vector>

#include "Framework/DataType.hpp"
#include "Tools/Math/Eigen.hpp"
#include "Tools/Time.hpp"

class GoalData : public DataType<GoalData> {
public:
  /// the name of this DataType
  DataTypeName name = "GoalData";
  /// the positions of detected goal postts
  VecVector2f posts;
  /// the timestamp of the image in which they were seen
  TimePoint timestamp;
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
