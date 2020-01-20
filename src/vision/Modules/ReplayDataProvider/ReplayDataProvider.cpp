#include "ReplayDataProvider.hpp"
#include "Modules/NaoProvider.h"
#include "Tools/Chronometer.hpp"

ReplayDataProvider::ReplayDataProvider(const ModuleManagerInterface& manager)
  : Module(manager)
  , fakeHeadMatrixBuffer_(*this, "fakeHeadMatrixBuffer", [] {})
  , head2torso_(*this, "head2torso", [this] { updateBuffer(); })
  , torso2ground_(*this, "torso2ground", [this] { updateBuffer(); })
  , cycleInfo_(*this)
  , headMatrixBuffer_(*this)
  , gameControllerState_(*this)
{
  updateBuffer();
}

void ReplayDataProvider::cycle()
{
  if (fakeHeadMatrixBuffer_())
  {
    restoreHeadMatrixBuffer();
  }
  gameControllerState_->packetNumber = gameControllerState_->packetNumber++;
  gameControllerState_->timestampOfLastMessage = TimePoint(0);
  gameControllerState_->playersPerTeam = 1;
  gameControllerState_->type = CompetitionType::NORMAL;
  gameControllerState_->competitionPhase = CompetitionPhase::ROUNDROBIN;
  gameControllerState_->gameState = GameState::PLAYING;
  gameControllerState_->gameStateChanged = TimePoint(0);
  gameControllerState_->gamePhase = GamePhase::NORMAL;
  gameControllerState_->setPlay = SetPlay::NONE;
  gameControllerState_->setPlayChanged = TimePoint(0);
  gameControllerState_->firstHalf = true;
  gameControllerState_->kickingTeam = true;
  gameControllerState_->kickingTeamNumber = 24;
  gameControllerState_->secondaryTime = 0.f;
  gameControllerState_->remainingTime = 0.f;
  gameControllerState_->teamColor = TeamColor::GRAY;
  gameControllerState_->score = 0;
  gameControllerState_->penalty = Penalty::NONE;
  gameControllerState_->remainingPenaltyTime = 0.f;
  gameControllerState_->chestButtonWasPressedInInitial = true;
  gameControllerState_->valid = true;
}

void ReplayDataProvider::restoreHeadMatrixBuffer()
{
  // Use replay data if available
  HeadMatrixBuffer hmb;
  if (robotInterface().getFakeData().getFakeData(hmb))
  {
    (*headMatrixBuffer_) = hmb;
    return;
  }
  else
  {
    headMatrixBuffer_->buffer.clear();
    headMatrixBuffer_->buffer.assign(buffer_.begin(), buffer_.end());
    headMatrixBuffer_->valid = true;
  }
}

void ReplayDataProvider::updateBuffer()
{
  buffer_[0].head2torso = head2torso_();
  buffer_[0].torso2ground = torso2ground_();
  buffer_[0].timestamp = TimePoint::getCurrentTime();
}
