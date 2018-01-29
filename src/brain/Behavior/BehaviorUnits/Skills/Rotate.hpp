#pragma once
#include "Behavior/Units.hpp"

ActionCommand rotate(const DataSet& d, bool left = true)
{
  return ActionCommand::walk(Pose(0, 0, static_cast<float>(M_PI / 4 * (left ? 1 : -1))));
}

ActionCommand rotate(const DataSet& d, float angle, bool absolute)
{
  const Pose target = absolute ? d.robotPosition.fieldToRobot(Pose(d.robotPosition.pose.position, angle)) : Pose(0, 0, angle);
  return walkToPose(d, target);
}

ActionCommand rotate(const DataSet& d, const Vector2f& target, const bool absolute = true)
{
  const Vector2f relTarget = absolute ? d.robotPosition.fieldToRobot(target) : target;
  return rotate(d, std::atan2(relTarget.y(), relTarget.x()), false);
}
