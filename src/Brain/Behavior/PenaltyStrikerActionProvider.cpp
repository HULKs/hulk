#include "Data/GameControllerState.hpp"
#include "Tools/BallUtils.hpp"
#include "Tools/Chronometer.hpp"
#include "Tools/Math/Random.hpp"

#include "Brain/Behavior/PenaltyStrikerActionProvider.hpp"


PenaltyStrikerActionProvider::PenaltyStrikerActionProvider(const ModuleManagerInterface& manager)
  : Module(manager)
  , ballState_(*this)
  , fieldDimensions_(*this)
  , gameControllerState_(*this)
  , robotPosition_(*this)
  , aimAtCornerFactor_(*this, "aimAtCornerFactor", [] {})
  , useOnlyThisFoot_(*this, "useOnlyThisFoot", [] {})
  , distanceToBallKick_(*this, "distanceToBallKick", [] {})
  , lastSign_(useOnlyThisFoot_())
  , penaltyTargetOffset_(0.f)
  , penaltyStrikerAction_(*this)
{
}

void PenaltyStrikerActionProvider::cycle()
{
  Chronometer time(debug(), mount_ + ".cycleTime");
  if ((gameControllerState_->gamePhase != GamePhase::PENALTYSHOOT &&
       gameControllerState_->setPlay != SetPlay::PENALTY_KICK) ||
      gameControllerState_->gameState != GameState::PLAYING || !gameControllerState_->kickingTeam ||
      gameControllerState_->penalty != Penalty::NONE)
  {
    penaltyTargetOffset_ = 0.f;
    return;
  }
  if (penaltyTargetOffset_ == 0.f)
  {
    // TODO: estimate a better target when there is a robot detection
    penaltyTargetOffset_ = Random::uniformInt(0, 1) == 0 ? -1.f : 1.f;
  }

  const Vector2f absBallPos = robotPosition_->robotToField(ballState_->position);
  const Vector2f penaltySpot = Vector2f(
      fieldDimensions_->fieldLength * 0.5f - fieldDimensions_->fieldPenaltyMarkerDistance, 0.f);

  const bool ballKickable = ballState_->found && (absBallPos - penaltySpot).norm() < 0.5f;
  if (ballKickable)
  {
    const Vector2f target = robotPosition_->fieldToRobot(Vector2f(
        fieldDimensions_->fieldLength * 0.5f,
        penaltyTargetOffset_ * fieldDimensions_->goalInnerWidth * 0.5f * aimAtCornerFactor_()));
    int useOnlyThisFoot = 1; // One may want to use useOnlyThisFoot_() here, but especially in
                             // penalty shootouts the left foot seems to be better.
    const bool forceSign = useOnlyThisFoot != 0;
    int& lastSign = forceSign ? useOnlyThisFoot : lastSign_;
    const float angleToBall = 5 * TO_RAD;
    const Pose kickPose =
        BallUtils::kickPose(ballState_->position, target, distanceToBallKick_().x(), lastSign,
                            forceSign, distanceToBallKick_().y());
    const BallUtils::Kickable kickable = BallUtils::kickable(
        kickPose, *ballState_, distanceToBallKick_().x(), angleToBall, distanceToBallKick_().y());
    penaltyStrikerAction_->kickPose = kickPose;
    penaltyStrikerAction_->type = PenaltyStrikerAction::Type::KICK;
    penaltyStrikerAction_->kickType = KickType::FORWARD;
    penaltyStrikerAction_->target = target;
    penaltyStrikerAction_->kickable = kickable;
    penaltyStrikerAction_->valid = true;
  }
  else
  {
    penaltyStrikerAction_->valid = false;
  }
}
