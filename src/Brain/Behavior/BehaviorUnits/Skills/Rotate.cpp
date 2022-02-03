#include "Brain/Behavior/Units.hpp"

ActionCommand rotate(const DataSet& /*d*/, bool left)
{
  return ActionCommand::walk(Pose(0, 0, static_cast<float>(M_PI / 4 * (left ? 1 : -1))));
}

ActionCommand rotate(const DataSet& d, float angle, bool isAbsolute)
{
  const Pose target{isAbsolute
                        ? d.robotPosition.fieldToRobot(Pose{d.robotPosition.pose.position(), angle})
                        : Pose{0, 0, angle}};
  return walkToPose(d, target);
}

ActionCommand rotate(const DataSet& d, const Vector2f& target, const bool isAbsolute)
{
  const Vector2f relTarget = isAbsolute ? d.robotPosition.fieldToRobot(target) : target;
  return rotate(d, std::atan2(relTarget.y(), relTarget.x()), false);
}
