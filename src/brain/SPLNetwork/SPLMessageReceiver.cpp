#include "Definitions/BHULKsStandardMessage.h"
#include "Definitions/SPLStandardMessage.h"

#include "BHULKsHelper.hpp"
#include "HULKsMessage.hpp"
#include "SPLMessageReceiver.hpp"

SPLMessageReceiver::SPLMessageReceiver(const ModuleManagerInterface& manager)
  : Module(manager, "SPLMessageReceiver")
  , playerConfiguration_(*this)
  , splNetworkData_(*this)
  , cycleInfo_(*this)
  , rawGameControllerState_(*this)
  , teamPlayers_(*this)
  , ntpData_(*this)
{
}

void SPLMessageReceiver::cycle()
{
  float dt = cycleInfo_->getTimeDiff(lastTime_);
  lastTime_ = cycleInfo_->startTime;
  // increase age and remove too old players
  for (auto it = internalPlayers_.players.begin(); it != internalPlayers_.players.end();)
  {
    it->age += dt;
    if (it->age > 3.f)
    {
      it = internalPlayers_.players.erase(it);
    }
    else
    {
      it++;
    }
  }
  // integrate incoming messages
  for (auto& it : splNetworkData_->messages)
  {
    const SPLStandardMessage& msg = it.first;
    // do not handle own messages and messages from penalized robots
    if (static_cast<unsigned int>(msg.playerNum) == playerConfiguration_->playerNumber)
    {
      continue;
    }
    TeamPlayer p;
    p.age = 0.0f;
    p.playerNumber = msg.playerNum;
    p.pose = Pose(msg.pose[0] * 0.001f, msg.pose[1] * 0.001f, msg.pose[2]);
    p.target = Vector2f(msg.walkingTo[0], msg.walkingTo[1]) * 0.001f;
    p.ballPosition = Vector2f(msg.ball[0], msg.ball[1]) * 0.001f;
    p.ballVelocity = Vector2f(msg.ballVel[0], msg.ballVel[1]) * 0.001f;
    if (msg.ballAge < 0.f || msg.ballAge * 1000 >= cycleInfo_->startTime.getSystemTime())
    {
      p.timeWhenBallWasSeen = 0;
    }
    else
    {
      p.timeWhenBallWasSeen = cycleInfo_->startTime - msg.ballAge * 1000;
    }
    p.fallen = (msg.fallen > 0);
    p.penalized = p.playerNumber <= rawGameControllerState_->penalties.size() && rawGameControllerState_->penalties[p.playerNumber - 1] != Penalty::NONE;
    // static_cast only works because the enum values are exactly the same as the ones from the message
    p.intention = static_cast<PlayerIntention>(msg.intention);
    if (playerConfiguration_->playerNumber <= SPL_STANDARD_MESSAGE_MAX_NUM_OF_PLAYERS)
    {
      p.suggestion = static_cast<PlayerSuggestion>(msg.suggestion[playerConfiguration_->playerNumber - 1]);
    }
    else
    {
      p.suggestion = PlayerSuggestion::NOTHING;
    }
    p.currentSideConfidence = static_cast<uint8_t>(msg.currentSideConfidence);
    p.currentPositionConfidence = static_cast<uint8_t>(msg.currentPositionConfidence);

    B_HULKs::BHULKsStandardMessage bhmsg;
    // This check is not completely safe. bhmsg.sizeOfBHULKsMessage returns the size of a message with no obstacles and no NTP messages.
    // If a malformatted message is received, bhmsg.read could read more bytes than allowed.
    if (msg.numOfDataBytes >= bhmsg.sizeOfBHULKsMessage() && bhmsg.read(msg.data))
    {
      const unsigned int receiveTime = it.second.getSystemTime();
      if (bhmsg.requestsNTPMessage)
      {
        NTPData::NTPRequest request;
        request.sender = msg.playerNum;
        request.origination = bhmsg.timestamp;
        request.receipt = receiveTime;
        ntpData_->ntpRequests.push_back(request);
      }
      for (auto& ntpMsg : bhmsg.ntpMessages)
      {
        if (ntpMsg.receiver == playerConfiguration_->playerNumber)
        {
          if (static_cast<unsigned int>(msg.playerNum) > ntpRobots_.size())
          {
            ntpRobots_.resize(msg.playerNum);
          }
          ntpRobots_[msg.playerNum - 1].offset = static_cast<int>(ntpMsg.requestReceipt - ntpMsg.requestOrigination + bhmsg.timestamp - receiveTime) / 2;
          ntpRobots_[msg.playerNum - 1].valid = true;
        }
      }
      // figure out whether robot is a HULK
      p.isHULK = (bhmsg.member == HULKS_MEMBER);
      // add local obstacles of the robot to the TeamPlayer
      p.localObstacles = bhmsg.obstacles;
      // convert obstacle centers back to meters because the B-HULKs message is based on millimeters
      for (auto& playerObstacle : p.localObstacles)
      {
        playerObstacle.center[0] *= 0.001f;
        playerObstacle.center[1] *= 0.001f;
      }
      // override with info from BHULKs message because it might be that the GameController does not know about a manually penalized robot
      p.penalized = bhmsg.isPenalized;
      p.keeperWantsToPlayBall = bhmsg.kingIsPlayingBall;
      p.currentPassTarget = bhmsg.passTarget;
      p.currentlyPerfomingRole = B_HULKs::bhulkToPlayingRole(bhmsg.currentlyPerfomingRole);
      p.roleAssignments.resize(BHULKS_STANDARD_MESSAGE_MAX_NUM_OF_PLAYERS);
      for (unsigned int i = 0; i < BHULKS_STANDARD_MESSAGE_MAX_NUM_OF_PLAYERS; i++)
      {
        p.roleAssignments[i] = B_HULKs::bhulkToPlayingRole(bhmsg.roleAssignments[i]);
      }
      p.headYaw = bhmsg.headYawAngle;
      if (ntpRobots_.size() >= static_cast<unsigned int>(msg.playerNum) && ntpRobots_[msg.playerNum - 1].valid)
      {
        p.timeWhenReachBall = std::max<int>(0, (bhmsg.timeWhenReachBall - ntpRobots_[msg.playerNum - 1].offset));
        p.timeWhenReachBallStriker = std::max<int>(0, (bhmsg.timeWhenReachBallQueen - ntpRobots_[msg.playerNum - 1].offset));
        p.timestampLastJumped = std::max<int>(0, (bhmsg.timestampLastJumped - ntpRobots_[msg.playerNum - 1].offset));
        p.lastTimeWhistleHeard = std::max<int>(0, (bhmsg.lastTimeWhistleDetected - ntpRobots_[msg.playerNum - 1].offset));
        p.timeWhenBallWasSeen = std::max<int>(0, (bhmsg.ballTimeWhenLastSeen - ntpRobots_[msg.playerNum - 1].offset));
      }
      else
      {
        p.timeWhenReachBall = cycleInfo_->startTime + 600000;
        p.timeWhenReachBallStriker = cycleInfo_->startTime + 600000;
        p.timestampLastJumped = 0;
        p.lastTimeWhistleHeard = 0;
      }

      // Here the hulks message is being processed if there is any.
      HULKs::HULKsMessage hulksMessage;
      if (msg.numOfDataBytes >= bhmsg.sizeOfBHULKsMessage() + hulksMessage.sizeOfHULKsMessage() && hulksMessage.read(msg.data + bhmsg.sizeOfBHULKsMessage()))
      {
        p.currentSearchPosition = hulksMessage.ballSearchData.currentSearchPosition;
        p.suggestedSearchPositions.resize(hulksMessage.ballSearchData.positionSuggestions.size());
        for (unsigned int i = 0; i < hulksMessage.ballSearchData.positionSuggestions.size(); i++)
        {
          p.suggestedSearchPositions[i] = hulksMessage.ballSearchData.positionSuggestions[i];
        }
      }
    }
    else
    {
      p.isHULK = false;
      p.currentlyPerfomingRole = PlayingRole::DEFENDER;
      p.headYaw = 0;
      p.timeWhenReachBall = cycleInfo_->startTime + 600000;
      p.timeWhenReachBallStriker = cycleInfo_->startTime + 600000;
      p.lastTimeWhistleHeard = 0;
      p.currentPassTarget = -1;
      p.keeperWantsToPlayBall = false;
    }
    bool merged = false;
    for (auto& it2 : internalPlayers_.players)
    {
      if (it2.playerNumber == p.playerNumber)
      {
        it2 = p;
        merged = true;
        break;
      }
    }
    if (!merged)
    {
      internalPlayers_.players.push_back(p);
    }
  }
  internalPlayers_.activePlayers = 0;
  internalPlayers_.activeHULKPlayers = 0;
  for (auto& player : internalPlayers_.players)
  {
    if (!player.penalized)
    {
      internalPlayers_.activePlayers++;
      if (player.isHULK)
      {
        internalPlayers_.activeHULKPlayers++;
      }
    }
  }
  *teamPlayers_ = internalPlayers_;

  debug().update(mount_ + ".TeamPlayers", *teamPlayers_);
}
