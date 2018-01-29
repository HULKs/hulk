#include "Tools/Chronometer.hpp"

#include "WorldStateProvider.hpp"


WorldStateProvider::WorldStateProvider(const ModuleManagerInterface& manager)
  : Module(manager, "WorldStateProvider")
  , robotPosition_(*this)
  , teamBallModel_(*this)
  , gameControllerState_(*this)
  , cycleInfo_(*this)
  , fieldDimensions_(*this)
  , worldState_(*this)
  , currentBallXThreshold_(0.f)
  , currentBallYThreshold_(0.f)
  , currentRobotXThreshold_(0.f)
  , currentRobotYThreshold_(0.f)
{
}

void WorldStateProvider::cycle()
{
  Chronometer time(debug(), mount_ + ".cycleTime");

  if (gameControllerState_->state == GameState::PLAYING)
  {
    if (!ballIsFree_)
    {
      if (gameControllerState_->kickoff || cycleInfo_->getTimeDiff(gameControllerState_->stateChanged) > 10.f ||
          (teamBallModel_->ballType != TeamBallModel::BallType::NONE && teamBallModel_->position.norm() > fieldDimensions_->fieldCenterCircleDiameter * 0.5f))
      {
        // TODO: The last check is not good. It has false positives (due to mislocalization) and false negatives (due to a ball becoming already free when it is
        // touched).
        ballIsFree_ = true;
      }
    }
  }
  else
  {
    ballIsFree_ = false;
  }
  worldState_->ballIsFree = ballIsFree_;

  if (teamBallModel_->ballType != TeamBallModel::BallType::NONE)
  {
    const bool ballInOwnHalf = teamBallModel_->position.x() < currentBallXThreshold_;
    const bool ballInLeftHalf = teamBallModel_->position.y() > currentBallYThreshold_;
    updateBallThresholds(ballInOwnHalf, ballInLeftHalf);
    worldState_->ballInOwnHalf = ballInOwnHalf;
    worldState_->ballInLeftHalf = ballInLeftHalf;
    worldState_->ballValid = true;
  }
  else
  {
    currentBallXThreshold_ = 0.f;
    currentBallYThreshold_ = 0.f;
  }

  if (robotPosition_->valid)
  {
    const bool robotInOwnHalf = robotPosition_->pose.position.x() < currentRobotXThreshold_;
    const bool robotInLeftHalf = robotPosition_->pose.position.y() > currentRobotYThreshold_;
    updateRobotThresholds(robotInOwnHalf, robotInLeftHalf);
    worldState_->robotInOwnHalf = robotInOwnHalf;
    worldState_->robotInLeftHalf = robotInLeftHalf;
    worldState_->robotValid = true;
  }
  else
  {
    currentRobotXThreshold_ = 0.f;
    currentRobotYThreshold_ = 0.f;
  }
}


void WorldStateProvider::updateBallThresholds(const bool ballInOwnHalf, const bool ballInLeftHalf)
{
  if (ballInOwnHalf)
  {
    currentBallXThreshold_ = 0.5f;
  }
  else
  {
    currentBallXThreshold_ = 0.f;
  }
  if (ballInLeftHalf)
  {
    currentBallYThreshold_ = -0.2f;
  }
  else
  {
    currentBallYThreshold_ = 0.2f;
  }
}

void WorldStateProvider::updateRobotThresholds(const bool robotInOwnHalf, const bool robotInLeftHalf)
{
  if (robotInOwnHalf)
  {
    currentRobotXThreshold_ = 0.f;
  }
  else
  {
    currentRobotXThreshold_ = -0.5f;
  }
  if (robotInLeftHalf)
  {
    currentRobotYThreshold_ = -0.2f;
  }
  else
  {
    currentRobotYThreshold_ = 0.2f;
  }
}
