#include "Tools/Chronometer.hpp"
#include "Tools/Math/Geometry.hpp"
#include "Tools/Math/Hysteresis.hpp"

#include "WorldStateProvider.hpp"

WorldStateProvider::WorldStateProvider(const ModuleManagerInterface& manager)
  : Module(manager)
  , robotPosition_(*this)
  , teamBallModel_(*this)
  , gameControllerState_(*this)
  , cycleInfo_(*this)
  , fieldDimensions_(*this)
  , worldState_(*this)
  , ballIsFree_(false)
  , ballInOwnHalf_(true)
  , ballInLeftHalf_(true)
  , ballInCorner_(false)
  , ballInPenaltyArea_(false)
  , ballIsToMyLeft_(true)
  , ballInCenterCircle_(true)
  , robotInOwnHalf_(true)
  , robotInLeftHalf_(true)
  , ballInCornerThreshold_(*this, "ballInCornerThreshold", [] {})
  , ballInCornerXThreshold_(*this, "ballInCornerXThreshold", [] {})
  , ballInCornerYThreshold_(*this, "ballInCornerYThreshold", [] {})
{
}

void WorldStateProvider::cycle()
{
  Chronometer time(debug(), mount_ + ".cycleTime");

  if (gameControllerState_->gameState == GameState::PLAYING)
  {
    if (!ballIsFree_)
    {
      // It is mandatory to use only found team balls here. Replacing part of this condition with
      // ballInCenterCircle_ is not sufficient.
      if (gameControllerState_->kickingTeam ||
          cycleInfo_->getTimeDiff(gameControllerState_->gameStateChanged) > 10.f ||
          (teamBallModel_->found && teamBallModel_->ballType != TeamBallModel::BallType::NONE &&
           teamBallModel_->position.norm() > fieldDimensions_->fieldCenterCircleDiameter * 0.5f))
      {
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
    ballInOwnHalf_ = Hysteresis<float>::smallerThan(teamBallModel_->position.x(), 0.f, hysteresis_,
                                                    ballInOwnHalf_);
    ballInLeftHalf_ = Hysteresis<float>::greaterThan(teamBallModel_->position.y(), 0.f, hysteresis_,
                                                     ballInLeftHalf_);
    ballInCorner_ = checkBallInCorner(teamBallModel_->position);
    ballInPenaltyArea_ =
        Hysteresis<float>::smallerThan(std::abs(teamBallModel_->position.x()),
                                       fieldDimensions_->fieldLength / 2 + hysteresis_, hysteresis_,
                                       ballInPenaltyArea_) &&
        Hysteresis<float>::greaterThan(std::abs(teamBallModel_->position.x()),
                                       fieldDimensions_->fieldLength / 2 -
                                           fieldDimensions_->fieldPenaltyAreaLength - hysteresis_,
                                       hysteresis_, ballInPenaltyArea_) &&
        Hysteresis<float>::smallerThan(std::abs(teamBallModel_->position.y()),
                                       fieldDimensions_->fieldPenaltyAreaWidth / 2 + hysteresis_,
                                       hysteresis_, ballInPenaltyArea_);
    ballIsToMyLeft_ = Hysteresis<float>::greaterThan(teamBallModel_->position.y(),
                                                     robotPosition_->pose.position.y(), hysteresis_,
                                                     ballIsToMyLeft_);
    ballInCenterCircle_ = Hysteresis<float>::smallerThan(teamBallModel_->position.norm(), fieldDimensions_->fieldCenterCircleDiameter / 2, hysteresis_, ballInCenterCircle_);
    worldState_->ballInOwnHalf = ballInOwnHalf_;
    worldState_->ballInLeftHalf = ballInLeftHalf_;
    worldState_->ballInCorner = ballInCorner_;
    worldState_->ballInPenaltyArea = ballInPenaltyArea_;
    worldState_->ballIsToMyLeft = ballIsToMyLeft_;
    worldState_->ballInCenterCircle = ballInCenterCircle_;
    worldState_->ballValid = true;
  }

  if (robotPosition_->valid)
  {
    robotInOwnHalf_ = Hysteresis<float>::smallerThan(robotPosition_->pose.position.x(), 0.f,
                                                     hysteresis_, robotInOwnHalf_);
    robotInLeftHalf_ = Hysteresis<float>::greaterThan(robotPosition_->pose.position.y(), 0.f,
                                                      hysteresis_, robotInLeftHalf_);
    worldState_->robotInOwnHalf = robotInOwnHalf_;
    worldState_->robotInLeftHalf = robotInLeftHalf_;
    worldState_->robotValid = true;
  }
}

bool WorldStateProvider::checkBallInCorner(const Vector2f& absBallPosition)
{
  const float currentBallInCornerThreshold = ballInCorner_ ? ballInCornerThreshold_() + hysteresis_
                                                           : ballInCornerThreshold_() - hysteresis_;
  Vector2f absCornerPosition =
      Vector2f(fieldDimensions_->fieldLength / 2, fieldDimensions_->fieldWidth / 2);
  if (Geometry::isInsideEllipse(absBallPosition, absCornerPosition, ballInCornerXThreshold_(),
                                ballInCornerYThreshold_(), currentBallInCornerThreshold))
  {
    return true;
  }
  absCornerPosition =
      Vector2f(-fieldDimensions_->fieldLength / 2, fieldDimensions_->fieldWidth / 2);
  if (Geometry::isInsideEllipse(absBallPosition, absCornerPosition, ballInCornerXThreshold_(),
                                ballInCornerYThreshold_(), currentBallInCornerThreshold))
  {
    return true;
  }
  absCornerPosition =
      Vector2f(-fieldDimensions_->fieldLength / 2, -fieldDimensions_->fieldWidth / 2);
  if (Geometry::isInsideEllipse(absBallPosition, absCornerPosition, ballInCornerXThreshold_(),
                                ballInCornerYThreshold_(), currentBallInCornerThreshold))
  {
    return true;
  }
  absCornerPosition =
      Vector2f(fieldDimensions_->fieldLength / 2, -fieldDimensions_->fieldWidth / 2);
  if (Geometry::isInsideEllipse(absBallPosition, absCornerPosition, ballInCornerXThreshold_(),
                                ballInCornerYThreshold_(), currentBallInCornerThreshold))
  {
    return true;
  }
  return false;
}
