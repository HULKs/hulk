#pragma once

#include "Data/MotionRequest.hpp"
#include "Tools/Math/Angle.hpp"
#include "Tools/Math/Pose.hpp"

namespace SelectWalkMode
{
  /**
   * @brief pathOrPathWithOrientation selects path or path with orientation walk mode depending on
   * distance and angle to target pose
   * @param target pose the relative target pose
   * @param distanceThreshold the distance threshold for path with orientation
   * @param angleThreshold the angle threshold for path with orientation
   * @return path with orientation walk mode if close to target, otherwise path walk mode
   */
  inline WalkMode pathOrPathWithOrientation(const Pose& targetPose,
                                            const float distanceThreshold = 1.5f,
                                            const float angleThreshold = 30 * TO_RAD)
  {
    return targetPose.position.squaredNorm() < distanceThreshold * distanceThreshold &&
                   std::abs(targetPose.orientation) < angleThreshold
               ? WalkMode::PATH_WITH_ORIENTATION
               : WalkMode::PATH;
  }
} // namespace SelectWalkMode
