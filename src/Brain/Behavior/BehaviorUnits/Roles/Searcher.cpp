#include "Brain/Behavior/Units.hpp"
#include "Tools/Math/Velocity.hpp"
#include "Tools/SelectWalkMode.hpp"

ActionCommand searcher(const DataSet& d)
{
  // only use the searcher position if it is valid
  if (!d.searcherPosition.ownSearchPoseValid)
  {
    Log<M_BRAIN>(LogLevel::WARNING) << "Invalid searcher position";
    return ActionCommand::stand().combineHead(activeVision(d, VisionMode::SEARCH_FOR_BALL));
  }
  const ActionCommand::Body::WalkMode walkMode =
      SelectWalkMode::pathOrPathWithOrientation(d.searcherPosition.pose);
  return walkToPose(d, d.searcherPosition.pose, true, walkMode)
      .combineHead(activeVision(d, VisionMode::SEARCH_FOR_BALL));
}
