#include "Tools/Chronometer.hpp"

#include "ActionCommand.hpp"
#include "BehaviorModule.hpp"
#include "Units.hpp"


BehaviorModule::BehaviorModule(const ModuleManagerInterface& manager)
  : Module(manager)
  , remoteMotionRequest_(*this, "remoteMotionRequest",
                         [this] {
                           std::lock_guard<std::mutex> lg(actualRemoteMotionRequestLock_);
                           actualRemoteMotionRequest_ = remoteMotionRequest_();
                         })
  , useRemoteMotionRequest_(*this, "useRemoteMotionRequest", [] {})
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
  , ballSearchPosition_(*this)
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
  , motionRequest_(*this)
  , eyeLEDRequest_(*this)
  , actionCommand_(ActionCommand::dead())
  , dataSet_(*this, *gameControllerState_, *ballState_, *robotPosition_, *bodyPose_,
             *playerConfiguration_, *playingRoles_, *motionState_, *headMotionOutput_,
             *sitDownOutput_, *teamBallModel_, *teamPlayers_, *fieldDimensions_, *strikerAction_,
             *penaltyStrikerAction_, *setPlayStrikerAction_, *keeperAction_, *penaltyKeeperAction_,
             *pointOfInterests_, *cycleInfo_, *setPosition_, *defenderAction_, *defendingPosition_,
             *bishopPosition_, *supportingPosition_, *replacementKeeperAction_, *buttonData_,
             *worldState_, *kickConfigurationData_, *ballSearchPosition_, *headPositionData_,
             actionCommand_)
{

  {
    // This is needed because callbacks are called asynchronously and a MotionRequest is large
    // enough that it is too dangerous.
    std::lock_guard<std::mutex> lg(actualRemoteMotionRequestLock_);
    actualRemoteMotionRequest_ = remoteMotionRequest_();
  }
  useRemoteMotionRequest_() = false;
}

void BehaviorModule::cycle()
{
  Chronometer time(debug(), mount_ + ".cycle_time");
  if (gameControllerState_->gameState == GameState::PLAYING &&
      gameControllerState_->penalty == Penalty::NONE && !bodyPose_->fallen &&
      useRemoteMotionRequest_())
  {
    std::lock_guard<std::mutex> lg(actualRemoteMotionRequestLock_);
    *motionRequest_ = actualRemoteMotionRequest_;
  }
  else
  {
    actionCommand_ = rootBehavior(dataSet_);
    actionCommand_.toMotionRequest(*motionRequest_);
    actionCommand_.toEyeLEDRequest(*eyeLEDRequest_);
  }
}
