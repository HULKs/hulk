#pragma once

#include <Framework/DataType.hpp>
#include <Tools/Math/Pose.hpp>
#include "MotionRequest.hpp"

// When needed, this DataType can later be extended with a member that holds the complete path,
// which might be useful for visualization.

class MotionPlannerOutput : public DataType<MotionPlannerOutput, MotionRequest>
{
public:
  /// the name of this DataType
  DataTypeName name = "MotionPlannerOutput";
};
