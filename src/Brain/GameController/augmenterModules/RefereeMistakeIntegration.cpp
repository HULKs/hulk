#include "Brain/GameController/augmenterModules/RefereeMistakeIntegration.hpp"

#include "Tools/Math/Hysteresis.hpp"

RefereeMistakeIntegration::RefereeMistakeIntegration(ModuleBase& module)
  : teamBallModel_(module)
  , bodyPose_(module)
  , playerConfiguration_(module)
  , ballInOwnHalf_(false)
  , footContactAverage_()
  , prevRawGcState_()
  , prevGcState_()
{
}

void RefereeMistakeIntegration::cycle(const RawGameControllerState& rawGcState,
                                      GameControllerState& gcState)
{
  // set ballInOwnHalf
  if (teamBallModel_->ballType != TeamBallModel::BallType::NONE)
  {
    ballInOwnHalf_ =
        Hysteresis::smallerThan(teamBallModel_->absPosition.x(), 0.f, hysteresis_, ballInOwnHalf_);
  }

  integrateTimeOutAdminMode(rawGcState, gcState);

  if (gcState.gameState != GameState::PLAYING)
  {
    return;
  }

  integrateEarlyUnpenalized(gcState);
  integrateCornerKick(rawGcState, gcState);
  integrateGoalFreeKick(rawGcState, gcState);

  prevRawGcState_ = rawGcState;
  prevGcState_ = gcState;
}

void RefereeMistakeIntegration::integrateTimeOutAdminMode(const RawGameControllerState& rawGcState,
                                                          GameControllerState& gcState)
{
  if (rawGcState.gamePhase == GamePhase::TIMEOUT)
  {
    gcState.gameState = GameState::INITIAL;
  }
}

void RefereeMistakeIntegration::integrateEarlyUnpenalized(GameControllerState& gcState)
{
  footContactAverage_.put(static_cast<uint8_t>(bodyPose_->footContact ? 1 : 0));

  // Check if we were "unpenalized"
  if (prevGcState_.penalty != Penalty::NONE && gcState.penalty == Penalty::NONE)
  {
    if (footContactAverage_.getAverage() < 0.9f || !bodyPose_->footContact)
    {
      // Keep the penalty until we have (safe) foot contact.
      gcState.penalty = prevGcState_.penalty;
    }
  }
}

void RefereeMistakeIntegration::integrateCornerKick(const RawGameControllerState& rawGcState,
                                                    GameControllerState& gcState)
{
  // only do something when we have an ongoing corner kick
  if (rawGcState.setPlay != SetPlay::CORNER_KICK)
  {
    return;
  }

  // Do not correct the game state if we haven't seen the ball!
  if (!teamBallModel_->seen)
  {
    return;
  }

  // According to the rules the ball gets placed in a corner next to the goal when the other team is
  // rewarded a corner kick. Check whether this was done correctly:

  if (rawGcState.kickingTeam && ballInOwnHalf_)
  {
    // The ball must not be in our half when we are the kicking team
    gcState.kickingTeam = false;
    gcState.kickingTeamNumber = 0;
  }
  else if (!rawGcState.kickingTeam && !ballInOwnHalf_)
  {
    // The ball must not be in the enemy half when they are the kicking team
    gcState.kickingTeam = true;
    gcState.kickingTeamNumber = static_cast<uint8_t>(playerConfiguration_->teamNumber);
  }
}

void RefereeMistakeIntegration::integrateGoalFreeKick(const RawGameControllerState& rawGcState,
                                                      GameControllerState& gcState)
{
  // only do something when we have an ongoing goal free kick
  if (rawGcState.setPlay != SetPlay::GOAL_KICK)
  {
    return;
  }

  // Do not correct the game state if we haven't seen the ball!
  if (!teamBallModel_->seen)
  {
    return;
  }

  // According to the rules the ball gets placed right before the penalty box of the team that is
  // rewarded a goal free kick. Check whether this was done correctly:

  if (rawGcState.kickingTeam && !ballInOwnHalf_)
  {
    // The ball must not be in the enemies half when we are the kicking team
    gcState.kickingTeam = false;
    gcState.kickingTeamNumber = 0;
  }
  else if (!rawGcState.kickingTeam && ballInOwnHalf_)
  {
    // The ball must not be in our half when they are the kicking team
    gcState.kickingTeam = true;
    gcState.kickingTeamNumber = static_cast<uint8_t>(playerConfiguration_->teamNumber);
  }
}
