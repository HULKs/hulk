#include "Tools/Chronometer.hpp"
#include "Tools/FieldDimensionUtils.hpp"
#include "Tools/Math/Geometry.hpp"
#include "Tools/Math/Hysteresis.hpp"

#include "Brain/Behavior/WorldStateProvider.hpp"


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
  , ballInGoalBoxArea_(false)
  , ballIsToMyLeft_(true)
  , ballInCenterCircle_(true)
  , robotInOwnHalf_(true)
  , robotInLeftHalf_(true)
  , robotInPenaltyArea_(false)
  , robotInGoalBoxArea_(false)
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
          cycleInfo_->getAbsoluteTimeDifference(gameControllerState_->gameStateChanged) > 10s ||
          (teamBallModel_->found && teamBallModel_->ballType != TeamBallModel::BallType::NONE &&
           teamBallModel_->absPosition.norm() > fieldDimensions_->fieldCenterCircleDiameter * 0.5f))
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
    ballInOwnHalf_ =
        Hysteresis::smallerThan(teamBallModel_->absPosition.x(), 0.f, hysteresis_, ballInOwnHalf_);
    ballInLeftHalf_ =
        Hysteresis::greaterThan(teamBallModel_->absPosition.y(), 0.f, hysteresis_, ballInLeftHalf_);
    ballInCorner_ = checkBallInCorner(teamBallModel_->absPosition);
    ballInPenaltyArea_ = FieldDimensionUtils::isInPenaltyArea(
        teamBallModel_->absPosition, fieldDimensions_, hysteresis_, ballInPenaltyArea_);
    ballInGoalBoxArea_ = FieldDimensionUtils::isInGoalBoxArea(
        teamBallModel_->absPosition, fieldDimensions_, hysteresis_, ballInGoalBoxArea_);
    ballIsToMyLeft_ = Hysteresis::greaterThan(
        teamBallModel_->absPosition.y(), robotPosition_->pose.y(), hysteresis_, ballIsToMyLeft_);
    ballInCenterCircle_ = Hysteresis::smallerThan(teamBallModel_->absPosition.norm(),
                                                  fieldDimensions_->fieldCenterCircleDiameter / 2,
                                                  hysteresis_, ballInCenterCircle_);

    worldState_->ballInOwnHalf = ballInOwnHalf_;
    worldState_->ballInLeftHalf = ballInLeftHalf_;
    worldState_->ballInCorner = ballInCorner_;
    worldState_->ballInPenaltyArea = ballInPenaltyArea_;
    worldState_->ballInGoalBoxArea = ballInGoalBoxArea_;
    worldState_->ballIsToMyLeft = ballIsToMyLeft_;
    worldState_->ballInCenterCircle = ballInCenterCircle_;
    worldState_->ballValid = true;
  }

  if (robotPosition_->valid)
  {
    robotInOwnHalf_ =
        Hysteresis::smallerThan(robotPosition_->pose.x(), 0.f, hysteresis_, robotInOwnHalf_);
    robotInLeftHalf_ =
        Hysteresis::greaterThan(robotPosition_->pose.y(), 0.f, hysteresis_, robotInLeftHalf_);
    robotInPenaltyArea_ = FieldDimensionUtils::isInPenaltyArea(
        robotPosition_->pose.position(), fieldDimensions_, hysteresis_, robotInPenaltyArea_);
    robotInGoalBoxArea_ = FieldDimensionUtils::isInGoalBoxArea(
        robotPosition_->pose.position(), fieldDimensions_, hysteresis_, robotInPenaltyArea_);

    worldState_->robotInOwnHalf = robotInOwnHalf_;
    worldState_->robotInLeftHalf = robotInLeftHalf_;
    worldState_->robotInPenaltyArea = robotInPenaltyArea_;
    worldState_->robotInGoalBoxArea = robotInGoalBoxArea_;
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
  return Geometry::isInsideEllipse(absBallPosition, absCornerPosition, ballInCornerXThreshold_(),
                                   ballInCornerYThreshold_(), currentBallInCornerThreshold);
}
