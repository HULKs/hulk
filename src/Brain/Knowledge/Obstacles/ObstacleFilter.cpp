#include "Brain/Knowledge/Obstacles/ObstacleFilter.hpp"
#include "Hardware/Definitions.hpp"

ObstacleFilter::ObstacleFilter(const ModuleManagerInterface& manager)
  : Module(manager)
  , enableSonar_(
        *this, "enableSonarPSOPair", [] {},
        [this]() { return gameControllerState_->gamePhase != GamePhase::PENALTYSHOOT; })
  , enableFootBumper_(
        *this, "enableFootBumperPSOPair", [] {},
        [this]() { return gameControllerState_->gamePhase != GamePhase::PENALTYSHOOT; })
  , enableRobotDetection_(*this, "enableRobotDetection", [] {})
  , ballRadius_(*this, "ballRadius", [this] { configChanged_ = true; })
  , freeKickAreaRadius_(*this, "freeKickAreaRadius", [this] { configChanged_ = true; })
  , goalPostRadius_(*this, "goalPostRadius", [this] { configChanged_ = true; })
  , robotRadius_(*this, "robotRadius", [this] { configChanged_ = true; })
  , fallenRobotRadius_(*this, "fallenRobotRadius", [this] { configChanged_ = true; })
  , unknownObstacleRadius_(*this, "unknownObstacleRadius", [this] { configChanged_ = true; })
  , ignoreSonarObstaclesBeyondDistance_(*this, "ignoreSonarObstaclesBeyondDistance", [] {})
  , playerConfiguration_(*this)
  , bodyPose_(*this)
  , fieldDimensions_(*this)
  , gameControllerState_(*this)
  , ballState_(*this)
  , teamBallModel_(*this)
  , filteredRobots_(*this)
  , robotPosition_(*this)
  , sonarData_(*this)
  , worldState_(*this)
  , footCollisionData_(*this)
  , obstacleData_(*this)
  , configChanged_(true)
{
}

void ObstacleFilter::cycle()
{
  processSonar();
  processFootBumper();
  processBall();
  processFreeKick();
  processRobots();
  updateObstacleData();
}

void ObstacleFilter::processFootBumper()
{
  if (!enableFootBumper_() || !footCollisionData_->valid)
  {
    return;
  }
  if (footCollisionData_->collision)
  {
    Vector2f obsCenter;
    obsCenter.x() = 0.05f;
    obsCenter.y() = .0f;
    obstacleData_->obstacles.emplace_back(obsCenter, unknownObstacleRadius_(),
                                          ObstacleType::UNKNOWN);
  }
}

void ObstacleFilter::processSonar()
{
  if (!enableSonar_() || bodyPose_->fallen)
  {
    return;
  }
  Vector2f obsLeft;
  Vector2f obsRight;
  Vector2f obsCenter;
  // get filtered sonar data
  float distanceLeft = sonarData_->filteredValues[Sonars::LEFT];
  float distanceRight = sonarData_->filteredValues[Sonars::LEFT];

  // If data is invalid or its distance exceeds the trustworthy radius ignore obstacles. Else, calc
  // position.
  bool hasObstacleLeft = distanceLeft > 0 &&
                         distanceLeft <= ignoreSonarObstaclesBeyondDistance_() &&
                         sonarData_->valid[Sonars::LEFT];
  bool hasObstacleRight = distanceRight > 0 &&
                          distanceRight <= ignoreSonarObstaclesBeyondDistance_() &&
                          sonarData_->valid[Sonars::RIGHT];

  // Estimate obstacle positions in front of the sensors which are angled to the sides
  // See http://doc.aldebaran.com/2-1/family/robots/sonar_robot.html for the concrete values
  if (hasObstacleLeft)
  {
    obsLeft.x() = distanceLeft * 0.9064f; // cos(25°)
    obsLeft.y() = distanceLeft * 0.4226f; // sin(25°)
  }
  if (hasObstacleRight)
  {
    obsRight.x() = distanceRight * 0.9064f;  // cos(25°)
    obsRight.y() = -distanceRight * 0.4226f; // sin(25°)
  }

  // decide if detected object is single object in front of robot.
  if (hasObstacleLeft && hasObstacleRight && (obsLeft - obsRight).norm() < 0.05)
  {
    obsCenter = ((obsLeft + obsRight) / 2); // calc center of both sides to get center of obstacle
    obstacleData_->obstacles.emplace_back(obsCenter, unknownObstacleRadius_(),
                                          ObstacleType::UNKNOWN);
  }
  else
  {
    if (hasObstacleLeft)
    {
      obstacleData_->obstacles.emplace_back(obsLeft, unknownObstacleRadius_(),
                                            ObstacleType::UNKNOWN);
    }
    if (hasObstacleRight)
    {
      obstacleData_->obstacles.emplace_back(obsRight, unknownObstacleRadius_(),
                                            ObstacleType::UNKNOWN);
    }
  }
}

void ObstacleFilter::processBall()
{
  if (!ballState_->found)
  {
    return;
  }
  obstacleData_->obstacles.emplace_back(ballState_->position, ballRadius_(), ObstacleType::BALL);
}

void ObstacleFilter::processFreeKick()
{
  if (gameControllerState_->setPlay == SetPlay::GOAL_KICK)
  {
    const float goalFreeKickBallPosX =
        fieldDimensions_->fieldLength / 2.f - fieldDimensions_->fieldGoalBoxAreaLength;
    const float goalFreeKickBallPosY = fieldDimensions_->fieldGoalBoxAreaWidth / 2.f;

    obstacleData_->obstacles.emplace_back(
        robotPosition_->fieldToRobot(Vector2f(goalFreeKickBallPosX, goalFreeKickBallPosY)),
        freeKickAreaRadius_(), ObstacleType::FREE_KICK_AREA);
    debug().update(mount_ + ".GoalFreeKickAreaLeft", *(obstacleData_->obstacles.end() - 1));

    obstacleData_->obstacles.emplace_back(
        robotPosition_->fieldToRobot(Vector2f(goalFreeKickBallPosX, -goalFreeKickBallPosY)),
        freeKickAreaRadius_(), ObstacleType::FREE_KICK_AREA);
    debug().update(mount_ + ".GoalFreeKickAreaRight", *(obstacleData_->obstacles.end() - 1));
  }

  if (gameControllerState_->setPlay == SetPlay::CORNER_KICK)
  {
    // Add the two obstacles next to our goal (for when the enemy has a corner kick)
    const Vector2f cornerKickBallPos =
        Vector2f(fieldDimensions_->fieldLength / -2.f, fieldDimensions_->fieldWidth / 2.f);

    obstacleData_->obstacles.emplace_back(robotPosition_->fieldToRobot(cornerKickBallPos),
                                          freeKickAreaRadius_(), ObstacleType::FREE_KICK_AREA);

    obstacleData_->obstacles.emplace_back(
        robotPosition_->fieldToRobot(cornerKickBallPos +
                                     Vector2f(0.f, -1.f * fieldDimensions_->fieldWidth)),
        freeKickAreaRadius_(), ObstacleType::FREE_KICK_AREA);
  }

  // Spawns an obstacle around the ball if the enemy has Kick in
  if (gameControllerState_->setPlay == SetPlay::KICK_IN && !gameControllerState_->kickingTeam)
  {
    if (!teamBallModel_->seen)
    {
      return;
    }
    obstacleData_->obstacles.emplace_back(teamBallModel_->relPosition, freeKickAreaRadius_(),
                                          ObstacleType::FREE_KICK_AREA);
  }

  if (gameControllerState_->setPlay != SetPlay::NONE && !gameControllerState_->kickingTeam)
  {
    if (!teamBallModel_->seen)
    {
      return;
    }
    obstacleData_->obstacles.emplace_back(teamBallModel_->relPosition, freeKickAreaRadius_(),
                                          ObstacleType::FREE_KICK_AREA);
  }
}

void ObstacleFilter::processRobots()
{
  if (!enableRobotDetection_() || !filteredRobots_->valid)
  {
    return;
  }
  // for now we simply forward the robot data since it is faked anyway
  auto numberOfRobots = filteredRobots_->robots.size();
  auto currentSize = obstacleData_->obstacles.size();
  obstacleData_->obstacles.reserve(numberOfRobots + currentSize);
  // add the robots to the list of obstacles
  for (const auto& otherRobot : filteredRobots_->robots)
  {
    obstacleData_->obstacles.emplace_back(otherRobot.position, robotRadius_(),
                                          ObstacleType::ANONYMOUS_ROBOT);
  }
}

void ObstacleFilter::updateObstacleData()
{
  if (configChanged_)
  {
    obstacleData_->typeRadius[static_cast<int>(ObstacleType::GOAL_POST)] = goalPostRadius_();
    obstacleData_->typeRadius[static_cast<int>(ObstacleType::UNKNOWN)] = unknownObstacleRadius_();
    obstacleData_->typeRadius[static_cast<int>(ObstacleType::ANONYMOUS_ROBOT)] = robotRadius_();
    obstacleData_->typeRadius[static_cast<int>(ObstacleType::HOSTILE_ROBOT)] = robotRadius_();
    obstacleData_->typeRadius[static_cast<int>(ObstacleType::TEAM_ROBOT)] = robotRadius_();
    obstacleData_->typeRadius[static_cast<int>(ObstacleType::FALLEN_ANONYMOUS_ROBOT)] =
        fallenRobotRadius_();
    obstacleData_->typeRadius[static_cast<int>(ObstacleType::FALLEN_HOSTILE_ROBOT)] =
        fallenRobotRadius_();
    obstacleData_->typeRadius[static_cast<int>(ObstacleType::FALLEN_TEAM_ROBOT)] =
        fallenRobotRadius_();
    obstacleData_->typeRadius[static_cast<int>(ObstacleType::BALL)] = ballRadius_();
    obstacleData_->typeRadius[static_cast<int>(ObstacleType::FREE_KICK_AREA)] =
        freeKickAreaRadius_();
    obstacleData_->typeRadius[static_cast<int>(ObstacleType::INVALID)] = unknownObstacleRadius_();
    configChanged_ = false;
  }
}
