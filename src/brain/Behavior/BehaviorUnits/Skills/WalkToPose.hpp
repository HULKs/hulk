#pragma once

#include "Behavior/Units.hpp"
#include <cmath>

/***
 * walkToPose calculates the walk request to a given position
 * It is checked if further movement is needed to be done regarding if robot is close to target
 * Walk commands which results into leaving the field movements are corrected. The target position is modified.
 *
 * @param a reference of type DataSet with actual infos about the robots environment and status
 * @param pose a reference of type Pose containing the coordinates and orientation of the target position
 * @param absolute a flag if pose coordinates are relative to actual robots position or absolute in field coordinates
 * @param walkingMode specifies the mode of operation for the motionplanner like following path with fixed orientation
 * @param velocity Desired walking velocities, movement and rotation. [m/s]
 * @param hysteresis the factor by which the target reached thresholds are multiplied if already standing
 * @param fallback the action command that is executed when not walking
 * @author a.hasselbring, g.felbinger, p.goettsch
 * @return calculated ActionCommand
 */
ActionCommand walkToPose(const DataSet& d, const Pose& pose, bool absolute = false, const WalkMode walkMode = WalkMode::PATH,
                         const Velocity& velocity = Velocity(), float hysteresis = 2.0, const ActionCommand fallback = ActionCommand::stand())
{
  assert(walkMode != WalkMode::VELOCITY); // Velocity mode doesn't make sense for this action

  Pose absTarget = absolute ? pose : d.robotPosition.robotToField(pose);
  const float maxDistanceToBorder = d.fieldDimensions.fieldBorderStripWidth / 2;

  if (std::abs(absTarget.position.x()) > (d.fieldDimensions.fieldLength / 2 + maxDistanceToBorder))
  {
    const float signX = absTarget.position.x() < 0.0f ? -1.0f : 1.0f;
    absTarget.position.x() = signX * (d.fieldDimensions.fieldLength / 2 + maxDistanceToBorder);
    absTarget.orientation = std::atan2(absTarget.position.y(), absTarget.position.x());
  }

  if (std::abs(absTarget.position.y()) > (d.fieldDimensions.fieldWidth / 2 + maxDistanceToBorder))
  {
    const float signY = absTarget.position.y() < 0.0f ? -1.0f : 1.0f;
    absTarget.position.y() = signY * (d.fieldDimensions.fieldWidth / 2 + maxDistanceToBorder);
    absTarget.orientation = std::atan2(absTarget.position.y(), absTarget.position.x());
  }

  const Pose relTarget = d.robotPosition.fieldToRobot(absTarget);

  const bool near = relTarget.position.squaredNorm() < 0.01f && std::abs(relTarget.orientation) < 3 * TO_RAD;
  const bool near2 = relTarget.position.squaredNorm() < 0.01f * hysteresis * hysteresis &&
                     std::abs(relTarget.orientation) < 3 * hysteresis * TO_RAD; // TODO: make angles great again

  if (near || (d.lastActionCommand.body().type() == MotionRequest::BodyMotion::STAND && near2))
  {
    return fallback;
  }

  return ActionCommand::walk(relTarget, walkMode, velocity);
}
