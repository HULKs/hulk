#include <numeric>

#include "Data/ObstacleData.hpp"
#include "Tools/Math/Angle.hpp"

#include "Brain/CollisionDetection/CollisionDetector.hpp"

CollisionDetector::CollisionDetector(const ModuleManagerInterface& manager)
  : Module(manager)
  , bufferIter_()
  , timeOfLastDetection_()
  , lastStates_()
  , timeHoldState_(*this, "timeHoldState", [] {})
  , obstacleRangeOfVision_(*this, "obstacleRangeOfVision", [] {})
  , collisionSafetyDistance_(*this, "collisionSafetyDistance", [] {})
  , sizeOfBuffer_(*this, "sizeOfBuffer", [] {})
  , teamObstacleData_(*this)
  , gameControllerState_(*this)
  , cycleInfo_(*this)
  , collisionDetectorData_(*this)
{
  lastStates_.resize(
      sizeOfBuffer_(),
      std::vector<bool>{false,
                        false}); // initialize the vector with correct size and default values.
}

void CollisionDetector::cycle()
{
  // set current state to false - "default"
  lastStates_[bufferIter_] = {false, false};

  // Only perform prediction when permitted
  if (gameControllerState_->gameState != GameState::SET &&
      gameControllerState_->penalty == Penalty::NONE &&
      gameControllerState_->gamePhase != GamePhase::PENALTYSHOOT)
  {
    predictCollisionsFromObstacles();
  }

  updateOutput();
  sendDebug();

  // increase iterator
  bufferIter_ = (bufferIter_ + 1) % lastStates_.size();
}

void CollisionDetector::predictCollisionsFromObstacles()
{
  for (const Obstacle& obstacle : teamObstacleData_->obstacles)
  {
    if (obstacle.type != ObstacleType::BALL && obstacle.type != ObstacleType::INVALID &&
        obstacle.type != ObstacleType::FREE_KICK_AREA &&
        obstacle.relativePosition.norm() < (obstacle.radius + collisionSafetyDistance_()))
    {
      const float relativeObstacleAngle =
          std::atan2(obstacle.relativePosition.y(), obstacle.relativePosition.x());

      if (relativeObstacleAngle > 0 && relativeObstacleAngle < M_PI / 2)
      { // check left cone
        lastStates_[bufferIter_][SIDE_LEFT] = true;
      }
      else if (relativeObstacleAngle <= 0 && relativeObstacleAngle > -M_PI / 2)
      { // check right cone
        lastStates_[bufferIter_][SIDE_RIGHT] = true;
      }
    }
  }
}

void CollisionDetector::updateOutput()
{
  // calc Indicator, used to accumulate last states
  std::array<unsigned int, 2> indicator;
  indicator.fill(0);

  for (std::vector<bool> i : lastStates_)
  { // sum all last states together
    indicator[SIDE_LEFT] += i[SIDE_LEFT];
    indicator[SIDE_RIGHT] += i[SIDE_RIGHT];
  }
  // set outputs
  if (indicator[SIDE_LEFT] > lastStates_.size() / 2)
  { // if half of the past cylces are positive
    collisionDetectorData_->collisionLeft = true;
    timeOfLastDetection_[SIDE_LEFT] = cycleInfo_->startTime;
  }
  if (indicator[SIDE_RIGHT] > lastStates_.size() / 2)
  { // if half of the past cylces are positive
    collisionDetectorData_->collisionRight = true;
    timeOfLastDetection_[SIDE_RIGHT] = cycleInfo_->startTime;
  }
  // duel
  collisionDetectorData_->duel =
      collisionDetectorData_->collisionLeft && collisionDetectorData_->collisionRight;

  // set long term outputs
  // left
  collisionDetectorData_->collisionLeftRigid =
      cycleInfo_->getAbsoluteTimeDifference(timeOfLastDetection_[SIDE_LEFT]) < timeHoldState_();
  // right
  collisionDetectorData_->collisionRightRigid =
      cycleInfo_->getAbsoluteTimeDifference(timeOfLastDetection_[SIDE_RIGHT]) < timeHoldState_();
  // dueling
  collisionDetectorData_->duelRigid =
      collisionDetectorData_->collisionLeftRigid && collisionDetectorData_->collisionRightRigid;
}

void CollisionDetector::sendDebug() const
{
  debug().update(mount_ + ".lastStates_", lastStates_);
}
