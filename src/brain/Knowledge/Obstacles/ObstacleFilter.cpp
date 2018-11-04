#include "ObstacleFilter.hpp"
#include "Modules/NaoProvider.h"
#include "Tools/Math/Angle.hpp"
#include "print.h"


ObstacleFilter::ObstacleFilter(const ModuleManagerInterface& manager)
  : Module(manager)
  , enableSonar_(*this, "enableSonar", [] {})
  , enableFootBumper_(*this, "enableFootBumper", [] {})
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
  , robotData_(*this)
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
  processRobotData();
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
  Vector2f obsLeft, obsRight, obsCenter;
  // get filtered sonar data
  float distanceLeft = sonarData_->filteredValues[SONARS::LEFT];
  float distanceRight = sonarData_->filteredValues[SONARS::RIGHT];

  // If data is invalid or its distance exceeds the trustworthy radius ignore obstacles. Else, calc position.
  bool hasObstacleLeft = distanceLeft > 0 && distanceLeft <= ignoreSonarObstaclesBeyondDistance_() && sonarData_->valid[SONARS::LEFT];
  bool hasObstacleRight =
      distanceRight > 0 && distanceRight <= ignoreSonarObstaclesBeyondDistance_() && sonarData_->valid[SONARS::RIGHT];

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
  if (gameControllerState_->setPlay == SetPlay::GOAL_FREE_KICK)
  {
    const float goalFreeKickBallPosX =
        (fieldDimensions_->fieldLength / 2.f) - fieldDimensions_->fieldPenaltyMarkerDistance;
    const float goalFreeKickBallPosY = fieldDimensions_->fieldPenaltyAreaWidth / 2.f;

    obstacleData_->obstacles.emplace_back(
        robotPosition_->fieldToRobot(Vector2f(goalFreeKickBallPosX, goalFreeKickBallPosY)),
        freeKickAreaRadius_(), ObstacleType::FREE_KICK_AREA);
    debug().update(mount_ + ".GoalFreeKickAreaLeft", *(obstacleData_->obstacles.end() - 1));

    obstacleData_->obstacles.emplace_back(
        robotPosition_->fieldToRobot(Vector2f(goalFreeKickBallPosX, -goalFreeKickBallPosY)),
        freeKickAreaRadius_(), ObstacleType::FREE_KICK_AREA);
    debug().update(mount_ + ".GoalFreeKickAreaRight", *(obstacleData_->obstacles.end() - 1));
  }

  if (gameControllerState_->setPlay != SetPlay::NONE && !gameControllerState_->kickingTeam)
  {
    if (!teamBallModel_->seen)
    {
      return;
    }

    // check if the GC made a mistake (giving the enemy a goal free kick while the ball is inside
    // our half.
    if (gameControllerState_->setPlay == SetPlay::GOAL_FREE_KICK && worldState_->ballInOwnHalf)
    {
      return;
    }

    const Vector2f relBallPos = Vector2f(robotPosition_->fieldToRobot(teamBallModel_->position));

    // obstacle is only added if the robot is not inside the obstacle.
    // This was done because the obstacle avoidance is not performing well
    // when the robot is inside an obstacle.
    // TODO: Remove this check as soon as path planning is able to deal with this.
    // 15 is a magic number to avoid a despawning obstacle when the robot is very close to it.
    if (relBallPos.norm() > freeKickAreaRadius_() - freeKickAreaRadius_() * 0.15f)
    {
      obstacleData_->obstacles.emplace_back(relBallPos, freeKickAreaRadius_(),
                                            ObstacleType::FREE_KICK_AREA);
      debug().update(mount_ + ".PushingFreeKickArea", *(obstacleData_->obstacles.end() - 1));
    }
  }
}

void ObstacleFilter::processRobotData()
{
  // for now we simply forward the robot data since it is faked anyway
  auto numberOfRobots = robotData_->positions.size();
  auto currentSize = obstacleData_->obstacles.size();
  obstacleData_->obstacles.reserve(numberOfRobots + currentSize);
  // add the robots to the list of obstacles
  for (auto& otherRobotPosition : robotData_->positions)
  {
    obstacleData_->obstacles.emplace_back(otherRobotPosition, robotRadius_(),
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
