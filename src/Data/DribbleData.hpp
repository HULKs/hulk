#pragma once

#include "Framework/DataType.hpp"
#include "Hardware/Clock.hpp"
#include "Tools/Math/Pose.hpp"

class DribbleData : public DataType<DribbleData>
{
public:
  /// the name of this DataType
  DataTypeName name__{"DribbleData"};
  /// the step size to request from walking
  Pose stepRequest;
  /// whether the request is actually dribbling
  bool isDribbling{false};
  /// whether the data of this DataType is valid
  bool valid{false};
  /**
   * @brief reset the DataType
   */
  void reset() override
  {
    stepRequest = Pose{};
    isDribbling = false;
    valid = false;
  }

  void toValue(Uni::Value& value) const override
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["stepRequest"] << stepRequest;
    value["isDribbling"] << isDribbling;
    value["valid"] << valid;
  }

  void fromValue(const Uni::Value& value) override
  {
    value["stepRequest"] >> stepRequest;
    value["isDribbling"] >> isDribbling;
    value["valid"] >> valid;
  }
};
