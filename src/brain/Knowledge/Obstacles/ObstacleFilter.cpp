#include "ObstacleFilter.hpp"
#include "Modules/NaoProvider.h"
#include "Tools/Math/Angle.hpp"
#include "print.h"


ObstacleFilter::ObstacleFilter(const ModuleManagerInterface& manager)
  : Module(manager, "ObstacleFilter")
  , enableSonar_(*this, "enableSonar", [] {})
  , ballObstacleRadius_(*this, "ballObstacleRadius", [] {})
  , sonarObstacleRadius_(*this, "sonarObstacleRadius", [] {})
  , ignoreSonarObstaclesBeyondDistance_(*this, "ignoreSonarObstaclesBeyondDistance", [] {})
  , motionRequest_(*this)
  , ballState_(*this)
  , sonarData_(*this)
  , obstacleData_(*this)
{
}

void ObstacleFilter::cycle()
{
  // Only check for obstacles if walking.
  if (motionRequest_->bodyMotion != MotionRequest::BodyMotion::WALK)
  {
    return;
  }
  processSonar();
  processBall();

  debug().update("Brain.ObstacleFilter.obstacleData", *obstacleData_);
}

void ObstacleFilter::processSonar()
{
  if (!enableSonar_())
  {
    return;
  }

  float distanceLeft = -1, distanceRight = -1;
  Vector2f obsLeft, obsRight, obsCenter;
  bool hasObstacleLeft = false, hasObstacleRight = false;

  // get filtered sonar data
  distanceLeft = sonarData_->sonarLeft;
  distanceRight = sonarData_->sonarRight;

  // if obstacle, calc position.
  if ((hasObstacleLeft = sonarGetIsObstacle(distanceLeft)))
  {
    obsLeft.x() = distanceLeft * 0.9064; // cos(25°)
    obsLeft.y() = distanceLeft * 0.4226; // sin(25°)
  }
  if ((hasObstacleRight = sonarGetIsObstacle(distanceRight)))
  {
    obsRight.x() = distanceRight * 0.9064;  // cos(25°)
    obsRight.y() = -distanceRight * 0.4226; // sin(25°)
  }

  // decide if detected object is single object in front of robot.
  if (hasObstacleLeft && hasObstacleRight && (obsLeft - obsRight).norm() < 0.05)
  {
    obsCenter = ((obsLeft + obsRight) / 2); // calc center of both sides to get center of obstacle
    obstacleData_->obstacles.emplace_back(Obstacle::SONAR, obsCenter, sonarObstacleRadius_());
  }
  else
  {
    if (hasObstacleLeft)
    {
      obstacleData_->obstacles.emplace_back(Obstacle::SONAR, obsLeft, sonarObstacleRadius_());
    }
    if (hasObstacleRight)
    {
      obstacleData_->obstacles.emplace_back(Obstacle::SONAR, obsRight, sonarObstacleRadius_());
    }
  }
}

void ObstacleFilter::processBall()
{
  if (!ballState_->found)
  {
    return;
  }
  obstacleData_->obstacles.emplace_back(Obstacle::BALL, ballState_->position, ballObstacleRadius_());
}

bool ObstacleFilter::sonarGetIsObstacle(const float distance) const
{
  // If distance exceeds trustworthy radius ignore obstacle.
  return !(distance > ignoreSonarObstaclesBeyondDistance_() || distance <= 0);
}
