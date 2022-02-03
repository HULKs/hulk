#include "Tools/Chronometer.hpp"

#include "Brain/Behavior/BehaviorModule.hpp"

#include "Brain/Behavior/Units.hpp"

BehaviorModule::BehaviorModule(const ModuleManagerInterface& manager)
  : Module(manager)
  , remoteActionCommand_(*this, "remoteActionCommand",
                         [this] {
                           std::lock_guard<std::mutex> lg(remoteActionCommandLock_);
                           actualRemoteActionCommand_ = remoteActionCommand_();
                         })
  , useRemoteActionCommand_(*this, "useRemoteActionCommand", [] {})
  , enableRemotePuppetMode_(*this, "enableRemotePuppetMode", [] {})
  , gameControllerState_(*this)
  , ballState_(*this)
  , robotPosition_(*this)
  , bodyPose_(*this)
  , playerConfiguration_(*this)
  , playingRoles_(*this)
  , motionState_(*this)
  , headPositionData_(*this)
  , headMotionOutput_(*this)
  , sitDownOutput_(*this)
  , teamBallModel_(*this)
  , teamPlayers_(*this)
  , searcherPosition_(*this)
  , fieldDimensions_(*this)
  , strikerAction_(*this)
  , penaltyStrikerAction_(*this)
  , setPlayStrikerAction_(*this)
  , kickConfigurationData_(*this)
  , keeperAction_(*this)
  , penaltyKeeperAction_(*this)
  , cycleInfo_(*this)
  , setPosition_(*this)
  , defenderAction_(*this)
  , defendingPosition_(*this)
  , bishopPosition_(*this)
  , supportingPosition_(*this)
  , replacementKeeperAction_(*this)
  , pointOfInterests_(*this)
  , buttonData_(*this)
  , worldState_(*this)
  , loserPosition_(*this)
  , actionCommand_(*this)
  , lastBodyMotionType_(ActionCommand::Body::MotionType::DEAD)
  , dataSet_(*this, *gameControllerState_, *ballState_, *robotPosition_, *bodyPose_,
             *playerConfiguration_, *playingRoles_, *motionState_, *headMotionOutput_,
             *sitDownOutput_, *teamBallModel_, *teamPlayers_, *searcherPosition_, *fieldDimensions_,
             *strikerAction_, *penaltyStrikerAction_, *setPlayStrikerAction_, *keeperAction_,
             *penaltyKeeperAction_, *pointOfInterests_, *cycleInfo_, *setPosition_,
             *defenderAction_, *defendingPosition_, *bishopPosition_, *supportingPosition_,
             *replacementKeeperAction_, *buttonData_, *worldState_, *kickConfigurationData_,
             *headPositionData_, *loserPosition_, lastBodyMotionType_)
  , actualRemoteActionCommand_(ActionCommand::dead())
{
  {
    // This is needed because callbacks are called asynchronously and a MotionRequest is large
    // enough that it is too dangerous.
    std::lock_guard<std::mutex> lg(remoteActionCommandLock_);
    actualRemoteActionCommand_ = remoteActionCommand_();
  }
  enableRemotePuppetMode_() = false;
}

void BehaviorModule::cycle()
{
  Chronometer time(debug(), mount_ + ".cycle_time");
  if (gameControllerState_->gameState == GameState::PLAYING &&
      gameControllerState_->penalty == Penalty::NONE && !bodyPose_->fallen &&
      useRemoteActionCommand_())
  {
    std::lock_guard<std::mutex> lg(remoteActionCommandLock_);
    *actionCommand_ = actualRemoteActionCommand_;
  }
  else if (enableRemotePuppetMode_() && gameControllerState_->gameState == GameState::INITIAL &&
           gameControllerState_->penalty == Penalty::NONE &&
           gameControllerState_->chestButtonWasPressedInInitial)
  {
    *actionCommand_ = ActionCommand::puppet();
  }
  else
  {
    *actionCommand_ = rootBehavior(dataSet_);
  }
  lastBodyMotionType_ = actionCommand_->body().type;
}
