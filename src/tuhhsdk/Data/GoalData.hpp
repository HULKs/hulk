#pragma once

#include <vector>

#include "Framework/DataType.hpp"
#include "Tools/Math/Eigen.hpp"
#include "Tools/Time.hpp"

class GoalData : public DataType<GoalData> {
public:
  /// the positions of detected goal postts
  VecVector2f posts;
  /// the timestamp of the image in which they were seen
  TimePoint timestamp;
  /// whether the goal posts are valid
  bool valid;
  /**
   * @brief reset sets the goal to a defined state
   */
  void reset()
  {
    valid = false;
    posts.clear();
  }

  virtual void toValue(Uni::Value& value) const
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["posts"] << posts;
    value["timestamp"] << timestamp;
    value["valid"] << valid;
  }

  virtual void fromValue(const Uni::Value& value)
  {
    value["posts"] >> posts;
    value["timestamp"] >> timestamp;
    value["valid"] >> valid;
  }
};
