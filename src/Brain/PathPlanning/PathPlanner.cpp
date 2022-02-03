#include "Brain/PathPlanning/PathPlanner.hpp"
#include "Framework/Log/Log.hpp"
#include "Tools/Chronometer.hpp"
#include "Tools/Math/Arc.hpp"
#include "Tools/Math/Geometry.hpp"
#include "Tools/Math/Line.hpp"
#include <cmath>
#include <variant>

PathPlanner::PathPlanner(const ModuleManagerInterface& manager)
  : Module(manager)
  , actionCommand_(*this)
  , teamBallModel_(*this)
  , playingRoles_(*this)
  , robotPosition_(*this)
  , teamObstacles_(*this)
  , pathPlannerData_(*this)
  , additionalObstacleOffset_(*this, "additionalObstacleOffset", [] {})
  , hybridAlignDistance_(*this, "hybridAlignDistance", [] {})
  , ignoreRobotObstacleDistance_(*this, "ignoreRobotObstacleDistance",
                                 [this] {
                                   ignoreRobotObstacleDistanceSquared_ =
                                       ignoreRobotObstacleDistance_() *
                                       ignoreRobotObstacleDistance_();
                                 })
  , maxObstacleDistance_(
        *this, "maxObstacleDistance",
        [this] { maxObstacleDistanceSquared_ = maxObstacleDistance_() * maxObstacleDistance_(); })
  , minPathEdgeLength_(*this, "minPathEdgeLength", [] {})
  , obstacleInflation_(*this, "obstacleInflation", [] {})
{
  maxObstacleDistanceSquared_ = maxObstacleDistance_() * maxObstacleDistance_();
  ignoreRobotObstacleDistanceSquared_ =
      ignoreRobotObstacleDistance_() * ignoreRobotObstacleDistance_();
}

void PathPlanner::cycle()
{
  Chronometer time(debug(), mount_ + "." + "cycle_time");

  // reset the id counting to start at 0 for each new cycle
  PathNodeFactory::resetCounter();

  const Pose targetPose = robotPosition_->robotToField(actionCommand_->body().walkTarget);

  const bool ignoreAllObstacles{
      actionCommand_->body().walkMode == ActionCommand::Body::WalkMode::DIRECT ||
      actionCommand_->body().walkMode == ActionCommand::Body::WalkMode::DIRECT_WITH_ORIENTATION};
  // Create PathObstacles from TeamObstacles
  auto pathObstacles = ignoreAllObstacles ? std::vector<PathObstacle>{}
                                          : createPathObstacles(robotPosition_->pose.position(),
                                                                targetPose.position());

  // Initialize the search by setting the start and target nodes
  const bool canStartSearch =
      setStartAndTargetNode(robotPosition_->pose.position(), targetPose.position(), pathObstacles);

  if (!canStartSearch)
  {
    // we cannot plan, fall back to the requested action command
    pathPlannerData_->nextRelativePathPose = actionCommand_->body().walkTarget;
    return;
  }
  // Try to find the optimal path and return it as a list of nodes
  const std::vector<std::shared_ptr<PathNode>> pathNodes = findPath();

  // Create the production from the found nodes
  producePath(pathNodes);
  pathPlannerData_->nextRelativePathPose =
      calculateNextPathPose(robotPosition_->fieldToRobot(targetPose));

  time.stop();
  // Send debug info
  sendDebug(pathObstacles, pathNodes);

  if (!pathNodes.empty())
  {
    aStarSearch_.freeSolutionNodes();
  }
}

std::vector<PathObstacle> PathPlanner::createPathObstacles(const Vector2f& start,
                                                           const Vector2f& target)
{
  std::vector<PathObstacle> pathObstacles;

  // Reserve memory for all teamobstacles
  pathObstacles.reserve(teamObstacles_->obstacles.size());

  const bool ignoreRobotObstacles =
      (actionCommand_->body().walkMode == ActionCommand::Body::WalkMode::DRIBBLE ||
       actionCommand_->body().walkMode == ActionCommand::Body::WalkMode::WALK_BEHIND_BALL) &&
      teamBallModel_->relPosition.squaredNorm() <= ignoreRobotObstacleDistanceSquared_;

  // Create the path obstacles
  for (const auto& obstacle : teamObstacles_->obstacles)
  {
    if (ignoreRobotObstacles && (obstacle.type == ObstacleType::HOSTILE_ROBOT ||
                                 obstacle.type == ObstacleType::ANONYMOUS_ROBOT ||
                                 obstacle.type == ObstacleType::TEAM_ROBOT))
    {
      continue;
    }

    // only add obstacles within a range
    if ((robotPosition_->pose.position() - obstacle.absolutePosition).squaredNorm() >
        maxObstacleDistanceSquared_)
    {
      continue;
    }
    const auto inflatedRadius = obstacle.radius + obstacleInflation_();
    const auto inflatedRadiusSquared = inflatedRadius * inflatedRadius;
    Vector2f offset = Vector2f::Zero();
    bool obstacleMovedFromRobot{false};
    bool obstacleMovedFromTarget{false};

    // Check whether the start position is inside an obstacle
    const Vector2f obstacleToPosition = start - obstacle.absolutePosition;
    if (obstacleToPosition.squaredNorm() < inflatedRadiusSquared)
    {
      // Move obstacle away from where we are
      // Calculate the offset by which the obstacle is to be moved
      offset = obstacleToPosition -
               (inflatedRadius + additionalObstacleOffset_()) * obstacleToPosition.normalized();
      obstacleMovedFromRobot = true;
    }

    // Check whether the target is inside an obstacle
    const Vector2f movedObstacleToTarget = target - (obstacle.absolutePosition + offset);
    if (movedObstacleToTarget.squaredNorm() < inflatedRadiusSquared)
    {
      // Move obstacle away if we want to go there
      // Calculate offset the obstacle is to be moved
      offset = movedObstacleToTarget -
               (inflatedRadius + additionalObstacleOffset_()) * movedObstacleToTarget.normalized();
      obstacleMovedFromTarget = true;
    }

    // If at this point, we are now inside the obstacle, just ignore it.
    if ((obstacleMovedFromRobot && obstacleMovedFromTarget) ||
        (obstacleMovedFromTarget &&
         (start - (obstacle.absolutePosition + offset)).squaredNorm() < inflatedRadiusSquared) ||
        (obstacleMovedFromRobot &&
         (target - (obstacle.absolutePosition + offset)).squaredNorm() < inflatedRadiusSquared))
    {
      const bool left =
          Geometry::isLeftOf(Vector2f{target - start}, Vector2f{obstacle.absolutePosition - start});
      pathObstacles.emplace_back(
          Geometry::getEquidistantPoint(start, target, inflatedRadius + additionalObstacleOffset_(),
                                        left),
          inflatedRadius);
      continue;
    }

    // obstacle radius is increased to make it possible to walk around them
    const Vector2f absoluteMovedPosition = obstacle.absolutePosition + offset;
    pathObstacles.emplace_back(absoluteMovedPosition, inflatedRadius);
  }

  return pathObstacles;
}

float PathPlanner::hybridAlignmentAngle(const Pose& targetPose,
                                        const float targetAlignDistance) const
{
  assert(hybridAlignDistance_() > targetAlignDistance);
  // The distance from robot origin to target can directly be obtained from coordinates of the
  // target pose because we are using relative coordinates.
  float distanceToTargetPose = targetPose.position().norm();

  // If the distance is to low we return the original orientation to avoid numerical problems.
  if (distanceToTargetPose < 2 * std::numeric_limits<float>::epsilon())
  {
    return targetPose.angle();
  }

  float targetFacingFactor = 0;
  if (distanceToTargetPose > hybridAlignDistance_())
  {
    targetFacingFactor = 1;
  }
  else if (distanceToTargetPose < targetAlignDistance)
  {
    targetFacingFactor = 0;
  }
  else
  {
    targetFacingFactor = (distanceToTargetPose - targetAlignDistance) /
                         (hybridAlignDistance_() - targetAlignDistance);
  }
  // Interpolate between facing the target and adopting the target pose orientation, to calculate
  // the rotation angle to be achieved. To do so, angle deviations are weighted according to the
  // previously calculated targetFacingFactor.
  const float targetFacingOrientation = std::atan2(targetPose.y(), targetPose.x());
  return targetFacingOrientation * targetFacingFactor +
         targetPose.angle() * (1.f - targetFacingFactor);
}

Pose PathPlanner::calculateNextPathPose(const Pose& target) const
{
  if (target.position().norm() < minPathEdgeLength_())
  {
    return target;
  }
  Vector2f followPathPosition{Vector2f::Zero()};

  // Take edges from path until an edge longer than a minimum length is found. We don't want to
  // follow edges where start and end are practically identical. The length of arcs is approximated
  // by subtracting start and end.
  for (const auto& edge : pathPlannerData_->path.edges)
  {
    float approximateLength = 0.0f;
    Vector2f edgeEndPosition{Vector2f::Zero()};

    // take next edge and check if it is an arc
    if (const auto* const arcPath = std::get_if<Arc<float>>(&edge))
    {
      approximateLength = (arcPath->end - arcPath->start).norm();
      const Vector2f arcOrthogonal(arcPath->start - arcPath->circle.center);
      const auto sign = arcPath->clockwise ? 1.0f : -1.0f;
      // this is not entirely true, but sufficient for now
      followPathPosition =
          arcPath->start + sign * Vector2f(arcOrthogonal.y(), -arcOrthogonal.x()).normalized();
      edgeEndPosition = robotPosition_->fieldToRobot(arcPath->end);
    }
    else if (const auto* const linePath = std::get_if<Line<float>>(&edge))
    {
      // edge is not an arc, it must be a line then
      approximateLength = (linePath->p2 - linePath->p1).norm();
      followPathPosition = linePath->p2;
      edgeEndPosition = robotPosition_->fieldToRobot(linePath->p2);
    }
    else
    {
      Log<M_BRAIN>(LogLevel::ERROR) << "Encountered invalid path segment, neither Line nor Arc!";
      assert(false);
      return target;
    }

    if (approximateLength >= minPathEdgeLength_())
    {
      const auto orientation = (actionCommand_->body().walkMode ==
                                    ActionCommand::Body::WalkMode::PATH_WITH_ORIENTATION ||
                                actionCommand_->body().walkMode ==
                                    ActionCommand::Body::WalkMode::DIRECT_WITH_ORIENTATION)
                                   ? Angle::normalized(target.angle())
                                   : hybridAlignmentAngle(target, 0.05f);
      return Pose{robotPosition_->fieldToRobot(followPathPosition), orientation};
    }
  }
  // no path found, fallback to walk target
  return target;
}

bool PathPlanner::setStartAndTargetNode(const Vector2f& startPosition,
                                        const Vector2f& targetPosition,
                                        std::vector<PathObstacle>& pathObstacles)
{
  if (startPosition == targetPosition)
  {
    return false;
  }

  // Create the start state
  std::shared_ptr<PathNode> nodeStart =
      PathNodeFactory::create(startPosition, pathObstacles, nullptr);

  // Define the target state
  std::shared_ptr<PathNode> nodeTarget = PathNodeFactory::create(
      targetPosition, pathObstacles, nullptr, std::weak_ptr<PathNode>(), true);

  // Set them
  aStarSearch_.setStartAndGoalNodes(nodeStart, nodeTarget);
  return true;
}

std::vector<std::shared_ptr<PathNode>> PathPlanner::findPath()
{
  unsigned int searchState = 0;
  do
  {
    // run a search step
    searchState = aStarSearch_.searchStep();
  } while (searchState == AStarSearch<PathNode>::SEARCH_STATE_SEARCHING);

  std::vector<std::shared_ptr<PathNode>> pathNodes;
  if (searchState == AStarSearch<PathNode>::SEARCH_STATE_SUCCEEDED)
  {
    std::shared_ptr<PathNode> node = aStarSearch_.getSolutionStart();
    while (node != nullptr)
    {
      pathNodes.push_back(node);
      node = aStarSearch_.getSolutionNext();
    }
  }
  return pathNodes;
}

void PathPlanner::producePath(const std::vector<std::shared_ptr<PathNode>>& pathNodes)
{
  if (pathNodes.empty())
  {
    return;
  }
  // Start producing new path
  PathNode* previousNode = pathNodes.front().get();

  // Make sure that the objects are saved without dynamic resizing
  pathPlannerData_->path.edges.reserve(pathNodes.size());

  for (auto nodeIt = std::next(pathNodes.begin()); nodeIt != pathNodes.end(); nodeIt++)
  {
    PathNode* currentNode = nodeIt->get();
    const Vector2f& previousCoords = previousNode->absolutePosition;
    const Vector2f& currentCoords = currentNode->absolutePosition;

    // If the nodes are on the same obstacle add an arc, otherwise add a line
    if (!pathPlannerData_->path.edges.empty() && (previousNode->connectedObstacle != nullptr) &&
        previousNode->connectedObstacle == currentNode->connectedObstacle)
    {
      // Get the arc
      Arc<float>& currentArc = previousNode->storedArcs[currentNode->id];

      // Instead of creating a new arc path on same obstacle merge the arc path with the previous
      // one.
      if (auto* previousArc = std::get_if<Arc<float>>(&pathPlannerData_->path.edges.back()))
      {
        // As all arcs are defined counterclockwise internally we might need to swap
        if (previousArc->end != currentArc.start)
        {
          std::swap(currentArc.start, currentArc.end);
          std::swap(currentArc.relStart, currentArc.relEnd);
          currentArc.clockwise = true;
        }
        const bool isPreviousArcClockwise = previousArc->clockwise;
        // The arc shouldn't change the direction
        if (currentArc.clockwise != isPreviousArcClockwise)
        {
          Log<M_BRAIN>(LogLevel::WARNING) << "clockwise != isPreviousArcClockwise";
        }
        if (previousArc->end != previousCoords)
        {
          Log<M_BRAIN>(LogLevel::WARNING) << "lastArc->arc.end != lastCoord";
        }
        previousArc->end = currentCoords;
        previousArc->relEnd = previousArc->end - previousArc->circle.center;
      }
      else if (auto* previousLine = std::get_if<Line<float>>(&pathPlannerData_->path.edges.back()))
      {
        // As all arcs are defined counterclockwise internally we might need to swap
        if (previousLine->p2 != currentArc.start)
        {
          std::swap(currentArc.start, currentArc.end);
          std::swap(currentArc.relStart, currentArc.relEnd);
          currentArc.clockwise = true;
        }

        // Previous edge was an line path, so don't merge them
        pathPlannerData_->path.edges.emplace_back(
            Arc<float>(currentArc.circle, currentArc.start, currentArc.end, currentArc.clockwise));
      }
      else
      {
        Log<M_BRAIN>(LogLevel::ERROR)
            << "Encountered unexpected Path segment. Neither Arc nor Line.";
        assert(false);
      }
    }
    else
    {
      // Create a line because our nodes are not on the same obstacle
      pathPlannerData_->path.edges.emplace_back(Line<float>(previousCoords, currentCoords));
    }
    previousNode = currentNode;
  }
  // Save the costs and target position of the path
  pathPlannerData_->path.cost = aStarSearch_.getSolutionCost();
  pathPlannerData_->path.absoluteTargetPosition = previousNode->absolutePosition;
  pathPlannerData_->path.absoluteStartPosition = robotPosition_->pose.position();
}

void PathPlanner::sendDebug(const std::vector<PathObstacle>& pathObstacles,
                            const std::vector<std::shared_ptr<PathNode>>& pathNodes) const
{
  // Send the created nodes
  const auto obstacleKey = mount_ + ".pathObstacles";
  if (debug().isSubscribed(obstacleKey))
  {
    debug().update(obstacleKey, pathObstacles);
  }
  const auto nodesKey = mount_ + ".pathNodes";
  if (debug().isSubscribed(nodesKey))
  {
    std::vector<PathNode> value;
    value.reserve(pathNodes.size());
    std::transform(pathNodes.cbegin(), pathNodes.cend(), std::back_inserter(value),
                   [](const auto& nodePtr) { return *nodePtr; });
    debug().update(nodesKey, value);
  }
}

void PathPlanner::printDebug()
{
  std::cout << "======================================" << std::endl;
  for (const auto& edge : pathPlannerData_->path.edges)
  {
    if (const auto* line = std::get_if<Line<float>>(&edge))
    {
      std::cout << "(" << line->p1.x() << "," << line->p1.y() << ")|(" << line->p2.x() << ","
                << line->p2.y() << ") - Line with length of: " << (line->p2 - line->p1).norm()
                << std::endl;
    }
    else if (const auto* arc = std::get_if<Arc<float>>(&edge))
    {
      // Calculate start and end positions relative to the center.
      const Vector2f relArcStart = arc->start - arc->circle.center;
      const Vector2f relArcEnd = arc->end - arc->circle.center;
      float angle = 0.f;
      [[maybe_unused]] const bool valid =
          Geometry::getAngleBetween(relArcStart, relArcEnd, angle, false);
      assert(valid);
      // If start is left of end and clockwise
      // or end is left of start and anti clockwise
      // return length of small arc
      std::string arcType;
      if ((Geometry::isLeftOf(relArcEnd, relArcStart) == arc->clockwise))
      {
        arcType = "short";
      }
      else
      {
        arcType = "long";
        angle = 2.f * static_cast<float>(M_PI) - angle;
      }
      std::cout << "(" << arc->start.x() << "," << arc->start.y() << ")|(" << arc->end.x() << ","
                << arc->end.y() << ") - Arc (" << arcType
                << ") with angle of: " << angle * 180 / M_PI
                << " and length of: " << angle * arc->circle.radius << std::endl;
    }
    else
    {
      Log<M_BRAIN>(LogLevel::ERROR) << "Neither Line nor Arc. I give up.";
      assert(false);
    }
  }
}
