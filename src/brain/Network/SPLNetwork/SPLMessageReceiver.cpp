#include "Definitions/DevilSmashStandardMessage.hpp"
#include "Definitions/SPLStandardMessage.h"

#include "DSHelper.hpp"
#include "HULKsMessage.hpp"
#include "SPLMessageReceiver.hpp"

#include "print.h"

SPLMessageReceiver::SPLMessageReceiver(const ModuleManagerInterface& manager)
  : Module(manager)
  , playerConfiguration_(*this)
  , networkServiceData_(*this)
  , splNetworkData_(*this)
  , cycleInfo_(*this)
  , rawGameControllerState_(*this)
  , rawTeamPlayers_(*this)
  , ntpData_(*this)
{
}

void SPLMessageReceiver::cycle()
{
  float dt = cycleInfo_->getTimeDiff(lastTime_);
  lastTime_ = cycleInfo_->startTime;
  // increase age and remove too old players
  for (auto it = internalPlayers_.rawPlayers.begin(); it != internalPlayers_.rawPlayers.end();)
  {
    it->age += dt;
    if (it->age > 3.f)
    {
      it = internalPlayers_.rawPlayers.erase(it);
    }
    else
    {
      it++;
    }
  }
  // integrate incoming messages
  for (auto& message : splNetworkData_->messages)
  {
    const SPLStandardMessage& msg = message.stdMsg;
    // do not handle own messages and messages
    if (static_cast<unsigned int>(msg.playerNum) == playerConfiguration_->playerNumber)
    {
#ifdef NAO
      bool itWasMe = false;
      for (const auto& interface : networkServiceData_->interfaces)
      {
        if (message.senderAddr.to_string() == interface.addressString)
        {
          itWasMe = true;
        }
      }
      if (!itWasMe)
      {
        Log(LogLevel::WARNING) << "Robot " << message.senderAddr.to_string()
                               << " also has player number " << playerConfiguration_->playerNumber
                               << ". Ignoring message.";
      }
#endif
      continue;
    }
    int remainingBytes = msg.numOfDataBytes;

    // Extract data from SPL standard message fields (no custom data included)
    RawTeamPlayer p;
    p.age = 0.0f;
    p.playerNumber = msg.playerNum;
    p.pose = Pose(msg.pose[0] * 0.001f, msg.pose[1] * 0.001f, msg.pose[2]);
    p.ballPosition = Vector2f(msg.ball[0], msg.ball[1]) * 0.001f;
    if (msg.ballAge < 0.f || msg.ballAge * 1000 >= cycleInfo_->startTime.getSystemTime())
    {
      p.timeWhenBallWasSeen = TimePoint(0);
    }
    else
    {
      p.timeWhenBallWasSeen = cycleInfo_->startTime - msg.ballAge * 1000;
    }
    p.fallen = (msg.fallen > 0);
    p.penalized = p.playerNumber <= rawGameControllerState_->penalties.size() &&
                  rawGameControllerState_->penalties[p.playerNumber - 1] != Penalty::NONE;

    // Parse the data array of the SPL standard message (DS msg and HULKs msg)
    auto bytesRead = parseDSMsg(msg, remainingBytes, message.receiveTimePoint, p);
    remainingBytes = remainingBytes - bytesRead;
    // Check if parsing DS message failed
    if (bytesRead == 0)
    {
      Log(LogLevel::WARNING) << "Unable to parse DevilSMASH message from player " << p.playerNumber;
    }
    else if (p.isHULK)
    {
      // Parse the hulks message
      // Note: In theory we are able to send a hulks message without a ds message. We might want to
      // move this out of this else case
      bytesRead = parseHULKMsg(msg, remainingBytes, p);
      remainingBytes = remainingBytes - bytesRead;
      // Check if parsing failed
      if (bytesRead == 0)
      {
        Log(LogLevel::WARNING) << "Unable to parse HULKs message from player " << p.playerNumber;
      }
    }

    // Check if there is any data left OR if we read too much data
    if (remainingBytes != 0 && p.isHULK)
    {
      p.isHULK = false;
      p.currentlyPerformingRole = PlayingRole::DEFENDER_LEFT;
      p.headYaw = 0;
      p.timeWhenReachBall = cycleInfo_->startTime + 600000;
      p.timeWhenReachBallStriker = cycleInfo_->startTime + 600000;
      p.lastTimeWhistleHeard = TimePoint(0);
      p.currentPassTarget = -1;

      Log(LogLevel::ERROR)
          << "Incoming SPL message data was not parsed correctly. Remaining number of bytes: "
          << remainingBytes;
    }

    bool merged = false;
    for (auto& it2 : internalPlayers_.rawPlayers)
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
      internalPlayers_.rawPlayers.push_back(p);
    }
  }
  internalPlayers_.activePlayers = 0;
  internalPlayers_.activeHULKPlayers = 0;
  for (auto& player : internalPlayers_.rawPlayers)
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
  *rawTeamPlayers_ = internalPlayers_;

  debug().update(mount_ + ".RawTeamPlayers", *rawTeamPlayers_);
}

unsigned int SPLMessageReceiver::parseDSMsg(const SPLStandardMessage& msg,
                                            unsigned int remainingBytes,
                                            const TimePoint& receiveTime, RawTeamPlayer& p)
{
  DevilSmash::StandardMessage devilSmashMsg;

  // Return if there is no data left
  if (remainingBytes == 0)
  {
    Log(LogLevel::INFO) << "Received a SPL msg without DevilSMASH msg in custom data field";
    return 0;
  }

  // return if we cannot parse the header safely
  if (remainingBytes < sizeof(devilSmashMsg.header))
  {
    Log(LogLevel::ERROR) << "RemainingBytes is smaller than size of DevilSMASH header. WTF";
    assert(false);
    return 0;
  }
  // This check is not completely safe. devilSmashMsg.sizeOfDSMessage returns the size of a message
  // with no obstacles and no NTP messages. If a malformatted message is received, bhmsg.read
  // could read more bytes than allowed.
  const bool minimumSizeReached =
      remainingBytes >= static_cast<unsigned int>(devilSmashMsg.sizeOfDSMessage());

  if (!minimumSizeReached)
  {
    Log(LogLevel::ERROR) << "sizeOfDSMessage > remaining SPL message bytes!";
    return 0;
  }

  if (!devilSmashMsg.read(msg.data))
  {
    // invalidate data that may have been written to p.
    p.isHULK = false;
    p.currentlyPerformingRole = PlayingRole::DEFENDER_LEFT;
    p.headYaw = 0;
    p.timeWhenReachBall = cycleInfo_->startTime + 600000;
    p.timeWhenReachBallStriker = cycleInfo_->startTime + 600000;
    p.lastTimeWhistleHeard = TimePoint(0);
    p.currentPassTarget = -1;

    Log(LogLevel::ERROR) << "Received a SPL msg with malformatted DevilSMASH msg!";
    return 0;
  }

  assert(minimumSizeReached);

  if (devilSmashMsg.requestsNTPMessage)
  {
    NTPData::NTPRequest request;
    request.sender = msg.playerNum;
    request.origination = devilSmashMsg.timestamp;
    request.receipt = receiveTime.getSystemTime();
    ntpData_->ntpRequests.push_back(request);
  }
  for (auto& ntpMsg : devilSmashMsg.ntpMessages)
  {
    if (ntpMsg.receiver == playerConfiguration_->playerNumber)
    {
      if (static_cast<unsigned int>(msg.playerNum) > ntpRobots_.size())
      {
        ntpRobots_.resize(msg.playerNum);
      }
      ntpRobots_[msg.playerNum - 1].offset =
          static_cast<int>(ntpMsg.requestReceipt - ntpMsg.requestOrigination +
                           devilSmashMsg.timestamp - receiveTime.getSystemTime()) /
          2;
      ntpRobots_[msg.playerNum - 1].valid = true;
    }
  }
  // figure out whether robot is a HULK
  p.isHULK = (devilSmashMsg.member == HULKS_MEMBER);
  // convert obstacle centers back to meters because the B-HULKs message is based on millimeters
  for (auto& playerObstacle : p.localObstacles)
  {
    playerObstacle.center[0] *= 0.001f;
    playerObstacle.center[1] *= 0.001f;
  }
  // override with info from BHULKs message because it might be that the GameController does not
  // know about a manually penalized robot
  p.penalized = devilSmashMsg.isPenalized;
  p.isPoseValid = devilSmashMsg.isRobotPoseValid;
  p.currentlyPerformingRole =
      DevilSmash::dsRoleToPlayingRole(devilSmashMsg.currentlyPerformingRole);
  p.roleAssignments.resize(DS_STANDARD_MESSAGE_MAX_NUM_PLAYERS);
  for (unsigned int i = 0; i < DS_STANDARD_MESSAGE_MAX_NUM_PLAYERS; i++)
  {
    p.roleAssignments[i] = DevilSmash::dsRoleToPlayingRole(devilSmashMsg.roleAssignments[i]);
  }
  p.ballVelocity =
      Vector2f(devilSmashMsg.ballVelocity[0] * 0.001f, devilSmashMsg.ballVelocity[1] * 0.001f);
  p.headYaw = devilSmashMsg.headYawAngle;
  if (ntpRobots_.size() >= static_cast<unsigned int>(msg.playerNum) &&
      ntpRobots_[msg.playerNum - 1].valid)
  {
    p.timeWhenReachBall =
        TimePoint(std::max<int>(0, (devilSmashMsg.timeWhenReachBall - ntpRobots_[msg.playerNum - 1].offset)));
    p.timeWhenReachBallStriker = TimePoint(std::max<int>(
        0, (devilSmashMsg.timeWhenReachBallStriker - ntpRobots_[msg.playerNum - 1].offset)));
    p.timestampLastJumped = TimePoint(std::max<int>(
        0, (devilSmashMsg.timestampLastJumped - ntpRobots_[msg.playerNum - 1].offset)));
    p.lastTimeWhistleHeard = TimePoint(std::max<int>(
        0, (devilSmashMsg.lastTimeWhistleDetected - ntpRobots_[msg.playerNum - 1].offset)));
    p.timeWhenBallWasSeen = TimePoint(std::max<int>(
        0, (devilSmashMsg.timeWhenBallLastSeen - ntpRobots_[msg.playerNum - 1].offset)));
  }
  else
  {
    p.timeWhenReachBall = cycleInfo_->startTime + 600000;
    p.timeWhenReachBallStriker = cycleInfo_->startTime + 600000;
    p.timestampLastJumped = TimePoint(0);
    p.lastTimeWhistleHeard = TimePoint(0);
  }

  return devilSmashMsg.sizeOfDSMessage();
}

unsigned int SPLMessageReceiver::parseHULKMsg(const SPLStandardMessage& msg,
                                              unsigned int remainingBytes, RawTeamPlayer& p)
{
  HULKs::HULKsMessage hulksMessage;

  // return if there is no data left
  if (remainingBytes == 0)
  {
    Log(LogLevel::INFO) << "Received a SPL msg without HULKs msg in custom data field";
    return 0;
  }

  // return if we cannot parse the header safely
  if (remainingBytes < sizeof(hulksMessage.header))
  {
    Log(LogLevel::ERROR) << "RemainingBytes is smaller than size of HULKs header. WTF";
    assert(false);
    return 0;
  }
  // This check is not completely safe. hulksMessage.sizeOfHULKsMessage returns the size of a
  // message with no obstacles and no NTP messages. If a malformatted message is received,
  // bhmsg.read could read more bytes than allowed.
  const bool minimumSizeReached =
      remainingBytes >= static_cast<unsigned int>(hulksMessage.sizeOfHULKsMessage());
  if (!minimumSizeReached)
  {
    Log(LogLevel::ERROR) << "sizeOfHULKsMessage > remaining SPL message bytes!";
    return 0;
  }

  if (!hulksMessage.read(msg.data + (msg.numOfDataBytes - remainingBytes)))
  {
    Log(LogLevel::ERROR) << "Received a SPL msg with malformatted HULKs msg!";
    return 0;
  }

  assert(minimumSizeReached);

  p.walkingTo = hulksMessage.walkingTo;
  p.currentPassTarget = hulksMessage.passTarget;
  // add local obstacles of the robot to the RawTeamPlayer
  p.localObstacles = hulksMessage.obstacles;
  for (auto& obstacle : p.localObstacles)
  {
    obstacle.center[0] *= 0.001f;
    obstacle.center[1] *= 0.001f;
  }
  p.currentSearchPosition = hulksMessage.ballSearchData.currentSearchPosition;
  p.isAvailableForBallSearch = hulksMessage.ballSearchData.availableForSearch;

  for (uint8_t i = 0; i < MAX_NUM_PLAYERS; i++)
  {
    p.suggestedSearchPositionsValidity[i] =
        static_cast<bool>(hulksMessage.ballSearchData.positionSuggestionsValidity & (1 << i));
    p.suggestedSearchPositions[i] = hulksMessage.ballSearchData.positionSuggestions[i];
  }

  if (ntpRobots_.size() >= static_cast<unsigned int>(msg.playerNum) &&
      ntpRobots_[msg.playerNum - 1].valid)
  {
    p.timestampBallSearchMapUnreliable =
        TimePoint(std::max<int>(0, (hulksMessage.ballSearchData.timestampBallSearchMapUnreliable -
                          ntpRobots_[msg.playerNum - 1].offset)));
  }

  p.mostWisePlayerNumber = hulksMessage.ballSearchData.mostWisePlayerNumber;

  return hulksMessage.sizeOfHULKsMessage();
}
