#include "Tools/Chronometer.hpp"

#include "PenaltyKeeperActionProvider.hpp"

PenaltyKeeperActionProvider::PenaltyKeeperActionProvider(const ModuleManagerInterface& manager)
  : Module(manager)
  , goalLineLenTolerance_(*this, "goalLineLenTolerance", [] {})
  , squatThreshold_(*this, "squatThreshold", [] {})
  , ballDestinationTolerance_(*this, "ballDestinationTolerance", [] {})
  , minBallDestinationToRobotThresh_(*this, "minBallDestinationToRobotThresh", [] {})
  , fieldDimensions_(*this)
  , ballState_(*this)
  , gameControllerState_(*this)
  , penaltyAction_(*this)
  , previousActionType_(PenaltyKeeperAction::Type::WAIT)
  , goalLineHalfWithTolerance_(fieldDimensions_->fieldPenaltyAreaWidth / 2.0 +
                               goalLineLenTolerance_())
{
}

void PenaltyKeeperActionProvider::cycle()
{
  Chronometer time(debug(), mount_ + ".cycle_time");
  /// Check for penalty shoot state AND if the player is NOT kickoff (= keeper)
  if (gameControllerState_->gamePhase != GamePhase::PENALTYSHOOT &&
      !gameControllerState_->kickingTeam)
  {
    return;
  }

  /// if the game state is not playing or penalized, reset to wait.
  if (gameControllerState_->gameState != GameState::PLAYING ||
      gameControllerState_->penalty != Penalty::NONE)
  {
    penaltyAction_->type = PenaltyKeeperAction::Type::WAIT;
    previousActionType_ = PenaltyKeeperAction::Type::WAIT;
    penaltyAction_->valid = true;
    return;
  }

  /// if we are in a non wait state, keep doing the current action
  if (previousActionType_ != PenaltyKeeperAction::Type::WAIT)
  {
    penaltyAction_->type = previousActionType_;
    penaltyAction_->valid = true;
    return;
  }

  penaltyAction_->type = PenaltyKeeperAction::Type::WAIT; // default

  if (ballState_->found)
  { /// No ballstate confidence check. We are reducing reliability.
    /// if ball destination will be at least minBallDestinationToRobotThresh_ or lesser from robot
    /// (x) and will be in fieldPenaltyAreaWidth/2 (y)
    if (ballState_->destination.x() < minBallDestinationToRobotThresh_())
    {
      Vector2<float> ballPosToDestDiff = ballState_->destination - ballState_->position;
      // lengthen the ball traj. vector = unitVec * (length + extraLen)
      ballPosToDestDiff =
          ballPosToDestDiff.normalized() * (ballPosToDestDiff.norm() + ballDestinationTolerance_());

      float goalLineDest =
          -ballPosToDestDiff.y() * ballState_->position.x() / ballPosToDestDiff.x();
      if (goalLineDest < goalLineHalfWithTolerance_)
      {
        if (std::abs(goalLineDest) < squatThreshold_())
        { // squat
          penaltyAction_->type = PenaltyKeeperAction::Type::SQUAT;
        }
        else
        {
          if (goalLineDest > 0)
          { // jump left.
            penaltyAction_->type = PenaltyKeeperAction::Type::JUMP_LEFT;
          }
          else
          {
            penaltyAction_->type = PenaltyKeeperAction::Type::JUMP_RIGHT;
          }
        }
      }
    }
  }
  previousActionType_ = penaltyAction_->type;
  penaltyAction_->valid = true;
}
