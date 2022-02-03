#include "Brain/Behavior/Units.hpp"
#include "Tools/SelectWalkMode.hpp"

ActionCommand loser(const DataSet& d)
{
  // do not use the loser position if it is not valid
  if (!d.loserPosition.valid)
  {
    Log<M_BRAIN>(LogLevel::WARNING) << "Invalid loser position";
    return ActionCommand::stand().combineHead(activeVision(d, VisionMode::SEARCH_FOR_BALL));
  }
  const ActionCommand::Body::WalkMode walkMode =
      SelectWalkMode::pathOrPathWithOrientation(d.loserPosition.pose, 1.5f, 90.f * TO_RAD);
  return walkToPose(d, d.loserPosition.pose, true, walkMode)
      .combineHead(activeVision(d, VisionMode::SEARCH_FOR_BALL));
}
