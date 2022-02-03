#include "Tools/Chronometer.hpp"

#include "Brain/Knowledge/Obstacles/TeamObstacleFilter.hpp"
#include "Brain/Network/SPLNetwork/HULKsMessageHelper.hpp"


TeamObstacleFilter::TeamObstacleFilter(const ModuleManagerInterface& manager)
  : Module(manager)
  , reclassifyUnknownObstacles_(*this, "reclassifyUnknownObstacles", [] {})
  , goalPostsAreObstacles_(*this, "goalPostsAreObstacles", [] {})
  , teamPlayersAreObstacles_(*this, "teamPlayersAreObstacles", [] {})
  , useOtherRobotsObstacles_(*this, "useOtherRobotsObstacles", [] {})
  , obstacleMergeRadiusSquared_(*this, "obstacleMergeRadiusSquared", [] {})
  , bodyPose_(*this)
  , gameControllerState_(*this)
  , obstacleData_(*this)
  , teamPlayers_(*this)
  , robotPosition_(*this)
  , fieldDimensions_(*this)
  , teamObstacleData_(*this)
{
}

void TeamObstacleFilter::cycle()
{
  Chronometer time(debug(), mount_ + ".cycle_time");
  if (gameControllerState_->penalty == Penalty::NONE)
  {
    // add goal posts as obstacles (from world model, not from visual goal post detection
    integrateMapObstacles();
    // add obstacles from local (own) obstacleFilter
    integrateLocalObstacles();
    // add the team players as obstacles
    integrateTeamPlayerKnowledge();
  }
  debug().update(mount_ + ".teamObstacleData", *teamObstacleData_);
}

void TeamObstacleFilter::integrateLocalObstacles()
{
  for (const auto& localObstacle : obstacleData_->obstacles)
  {
    updateObstacle(localObstacle.relativePosition, robotPosition_->pose, localObstacle.type, false);
  }
}

void TeamObstacleFilter::integrateTeamPlayerKnowledge()
{
  for (const auto& teamPlayer : teamPlayers_->players)
  {
    if (teamPlayer.penalized)
    {
      continue;
    }
    // TODO: Maybe check, whether this teamPlayer is still on the field (illegal motion in set)
    if (teamPlayersAreObstacles_())
    {
      updateObstacle(teamPlayer.pose.position(), {0, 0, 0},
                     teamPlayer.fallen ? ObstacleType::FALLEN_TEAM_ROBOT : ObstacleType::TEAM_ROBOT,
                     false);
    }
    // Integrate obstacles detected by other robots
    if (!teamPlayer.fallen && useOtherRobotsObstacles_())
    {
      integrateTeamPlayersObstacles(teamPlayer);
    }
  }
}

void TeamObstacleFilter::integrateTeamPlayersObstacles(const TeamPlayer& teamPlayer)
{
  for (const auto& remoteObstacle : teamPlayer.localObstacles)
  {
    ObstacleType teamObstacleType = HULKs::hmObstacleTypeToObstacleType(remoteObstacle.type);
    if (teamObstacleType == ObstacleType::GOAL_POST)
    {
      // goal posts need to be skipped, because they are being integrated from the map
      continue;
    }
    updateObstacle({remoteObstacle.center[0], remoteObstacle.center[1]}, teamPlayer.pose,
                   teamObstacleType);
  }
}

void TeamObstacleFilter::integrateMapObstacles()
{
  if (!goalPostsAreObstacles_())
  {
    return;
  }
  // the goal post positions:
  const Vector2f ownGoalPostL = {
      -fieldDimensions_->fieldLength * 0.5f,
      (fieldDimensions_->goalInnerWidth + fieldDimensions_->goalPostDiameter) * 0.5f};
  const Vector2f ownGoalPostR = {ownGoalPostL.x(), -ownGoalPostL.y()};
  const Vector2f opponentGoalPostL = -ownGoalPostR;
  const Vector2f opponentGoalPostR = -ownGoalPostL;

  updateObstacle(ownGoalPostL, {0, 0, 0}, ObstacleType::GOAL_POST, false);
  updateObstacle(ownGoalPostR, {0, 0, 0}, ObstacleType::GOAL_POST, false);
  updateObstacle(opponentGoalPostL, {0, 0, 0}, ObstacleType::GOAL_POST, false);
  updateObstacle(opponentGoalPostR, {0, 0, 0}, ObstacleType::GOAL_POST, false);
}

bool TeamObstacleFilter::typeIsAtLeastAsSpecificAndMergable(const ObstacleType first,
                                                            const ObstacleType second) const
{
  return second == first || // The types are the same
         (reclassifyUnknownObstacles_() && second == ObstacleType::UNKNOWN &&
          first != ObstacleType::UNKNOWN && first != ObstacleType::BALL &&
          first != ObstacleType::FREE_KICK_AREA) ||
         (second == ObstacleType::ANONYMOUS_ROBOT &&
          (first == ObstacleType::HOSTILE_ROBOT || first == ObstacleType::TEAM_ROBOT)) ||
         (second == ObstacleType::FALLEN_ANONYMOUS_ROBOT &&
          (first == ObstacleType::FALLEN_HOSTILE_ROBOT ||
           first == ObstacleType::FALLEN_TEAM_ROBOT));
}

ObstacleType TeamObstacleFilter::mapToMergedType(const ObstacleType t1, const ObstacleType t2) const
{
  if (typeIsAtLeastAsSpecificAndMergable(t1, t2))
  {
    // t1 has a higherOrSame level specification. Thus this should be the type of the merged
    // obstacle
    return t1;
  }
  if (typeIsAtLeastAsSpecificAndMergable(t2, t1))
  {
    // t2 has a higher level specification. Thus this should be the type of the merged obstacle
    return t2;
  }
  // types are not mergeable
  return ObstacleType::INVALID;
}

bool TeamObstacleFilter::obstacleTypeIsCompatibleWithThisRobot(
    const ObstacleType obstacleType) const
{
  switch (obstacleType)
  {
    case ObstacleType::UNKNOWN:
      // unknown obstacle could be this robot
      return true;
    case ObstacleType::ANONYMOUS_ROBOT:
    case ObstacleType::TEAM_ROBOT:
      // non-fallen teammates and annonymous robots can be this robot, if this robot isn't fallen
      return !bodyPose_->fallen;
    case ObstacleType::FALLEN_ANONYMOUS_ROBOT:
    case ObstacleType::FALLEN_TEAM_ROBOT:
      // fallen teammates and annonymous robots can be this robot, if this robot is fallen
      return bodyPose_->fallen;
    default:
      return false;
  }
}

void TeamObstacleFilter::updateObstacle(const Vector2f& newObstaclePosition,
                                        const Pose& referencePose, const ObstacleType newType,
                                        const bool obstacleCouldBeThisRobot)
{
  assert(newType != ObstacleType::INVALID && newType != ObstacleType::OBSTACLETYPE_MAX);

  const Vector2f newAbsolutePosition = referencePose * newObstaclePosition;
  const Vector2f newRelativePosition = robotPosition_->fieldToRobot(newAbsolutePosition);

  auto closestMergableObstacle = teamObstacleData_->obstacles.end();
  ObstacleType typeIfMergedWithClosestObstacle = ObstacleType::INVALID;
  float distanceToClosestMergableObstacleSquared = obstacleMergeRadiusSquared_();
  for (auto itObstacle = teamObstacleData_->obstacles.begin();
       itObstacle != teamObstacleData_->obstacles.end(); itObstacle++)
  {
    const float itObstacleDistanceSquared =
        (newRelativePosition - itObstacle->relativePosition).squaredNorm();
    if (itObstacleDistanceSquared < distanceToClosestMergableObstacleSquared)
    {
      // figure out the merged type (invalid if not mergable)
      ObstacleType mergedObstacleType = mapToMergedType(newType, itObstacle->type);
      if (mergedObstacleType == ObstacleType::INVALID)
      {
        continue;
      }
      // merge valid and currently the best match:
      closestMergableObstacle = itObstacle;
      typeIfMergedWithClosestObstacle = mergedObstacleType;
      distanceToClosestMergableObstacleSquared = itObstacleDistanceSquared;
    }
  }
  // check whether the detected obstacle is this very robot or a goal post
  if ((obstacleCouldBeThisRobot &&
       newRelativePosition.squaredNorm() < distanceToClosestMergableObstacleSquared &&
       obstacleTypeIsCompatibleWithThisRobot(newType)) ||
      (typeIfMergedWithClosestObstacle == ObstacleType::GOAL_POST))
  {
    // this robot shouldn't be part of the obstacles. Otherwise it might to react to itself as an
    // obstacle and goal posts should be moved
    return;
  }
  // the obstacle can be merged with another valid obstacle
  if (closestMergableObstacle != teamObstacleData_->obstacles.end())
  {
    assert(typeIfMergedWithClosestObstacle != ObstacleType::INVALID && "Merge was invalid");
    // the position of the merged obstacle is averaged
    closestMergableObstacle->absolutePosition =
        closestMergableObstacle->absolutePosition * 0.5f + newAbsolutePosition * 0.5f;
    closestMergableObstacle->relativePosition =
        closestMergableObstacle->relativePosition * 0.5f + newRelativePosition * 0.5f;
    closestMergableObstacle->radius = obstacleData_->typeToRadius(typeIfMergedWithClosestObstacle);
    closestMergableObstacle->type = typeIfMergedWithClosestObstacle;
    return;
  }
  // the Obstacle could not be merged, thus a new one is added to obstacle vector
  const auto newRadius = obstacleData_->typeToRadius(newType);
  teamObstacleData_->obstacles.emplace_back(newRelativePosition, newAbsolutePosition, newRadius,
                                            newType);
}
