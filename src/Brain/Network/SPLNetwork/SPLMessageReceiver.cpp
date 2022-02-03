#include "Brain/Network/SPLNetwork/SPLMessageReceiver.hpp"
#include "Brain/Network/SPLNetwork/DSHelper.hpp"
#include "Brain/Network/SPLNetwork/HULKsMessage.hpp"
#include "Framework/Log/Log.hpp"
#include "Messages/DevilSmashStandardMessage.hpp"
#include "Messages/SPLStandardMessage.hpp"
#include <chrono>

SPLMessageReceiver::SPLMessageReceiver(const ModuleManagerInterface& manager)
  : Module(manager)
  , enablePlayerNumberWarning_(*this, "enablePlayerNumberWarning", [] {})
  , playerConfiguration_(*this)
  , networkServiceData_(*this)
  , splNetworkData_(*this)
  , cycleInfo_(*this)
  , rawGameControllerState_(*this)
  , rawTeamPlayers_(*this)
  , ntpData_(*this)
{
}

// NOLINTNEXTLINE(readability-function-cognitive-complexity)
void SPLMessageReceiver::cycle()
{
  // increase age and remove too old players
  for (auto it = internalPlayers_.rawPlayers.begin(); it != internalPlayers_.rawPlayers.end();)
  {
    it->age += cycleInfo_->cycleTime;
    if (it->age > 3s)
    {
      it = internalPlayers_.rawPlayers.erase(it);
      Log<M_BRAIN>(LogLevel::WARNING)
          << "Team Player " << it->playerNumber << " was removed (timeout).";
    }
    else
    {
      it++;
    }
  }
  // integrate incoming messages
  for (const auto& message : splNetworkData_->messages)
  {
    const SPLStandardMessage::SPLStandardMessage& msg = message.message;
    // do not handle own messages and messages
    if (static_cast<unsigned int>(msg.playerNum) == playerConfiguration_->playerNumber)
    {
#ifdef HULK_TARGET_NAO
      // check if there is some other nao with the same player number on the network and give some
      // audio feedback. This is only possible on the NAO as the networkServiceData_ is only
      // available on the NAO
      bool itWasMe = false;
      for (const auto& interface : networkServiceData_->interfaces)
      {
        if (message.senderAddress.to_string() == interface.addressString)
        {
          itWasMe = true;
        }
      }
      if (!itWasMe)
      {
        Log<M_BRAIN>(LogLevel::WARNING)
            << "Robot " << message.senderAddress.to_string() << " also has player number "
            << playerConfiguration_->playerNumber << ". Ignoring message.";

        if (!enablePlayerNumberWarning_())
        {
          continue;
        }
        // parse the ip of the sender
        std::string ipString = message.senderAddress.to_string();
        std::array<std::uint32_t, 4> ipBytes{};
        // we know that the IP is of format #.#.#.#. Using sscanf is safe.
        // NOLINTNEXTLINE(hicpp-vararg, cert-err34-c)
        sscanf(ipString.c_str(), "%u.%u.%u.%u", ipBytes.data(), ipBytes.data() + 1,
               ipBytes.data() + 2, ipBytes.data() + 3);

        const bool networkMatches = ipBytes[0] == 10;
        const bool isETHSubnet = ipBytes[1] == 1;
        const bool isWIFISubnet = ipBytes[1] == 0;
        const bool isOwnTeamSubnet = ipBytes[2] == playerConfiguration_->teamNumber;

        // check if the sender is in our subnet
        if (networkMatches && (isETHSubnet || isWIFISubnet) && isOwnTeamSubnet)
        {
          // check if we have the lower nao number (only the one with the lower nao number should
          // output sound), continue if not
          const int remoteNaoNumber = std::stoi(ipString.substr(ipString.length() - 2)) - 10;
          const auto& localNaoInfo = robotInterface().getRobotInfo();
          const int ownNaoNumber =
              std::stoi(localNaoInfo.headName.substr(localNaoInfo.headName.length() - 2));
          if (ownNaoNumber > remoteNaoNumber)
          {
            continue;
          }
          // get the correct audio file for the job.
          const int audioFileNumber =
              static_cast<int>(AudioSounds::SAME_PLAYER_NUMBER_MIN) + remoteNaoNumber - 20;
          if (audioFileNumber > static_cast<int>(AudioSounds::SAME_PLAYER_NUMBER_MIN) &&
              audioFileNumber < static_cast<int>(AudioSounds::SAME_PLAYER_NUMBER_MAX))
          {
            debug().playAudio("same player number NAO", static_cast<AudioSounds>(audioFileNumber));
          }
          else if (isETHSubnet)
          {
            debug().playAudio("same player number eth",
                              AudioSounds::SAME_PLAYER_NUMBER_GENERAL_ETH);
          }
          else if (isWIFISubnet)
          {
            debug().playAudio("same player number wifi",
                              AudioSounds::SAME_PLAYER_NUMBER_GENERAL_WIFI);
          }
        }
      }
#endif
      continue;
    }
    int remainingBytes = msg.numOfDataBytes;

    // Extract data from SPL standard message fields (no custom data included)
    RawTeamPlayer player;
    player.age = 0s;
    player.playerNumber = msg.playerNum;
    player.pose = Pose(msg.pose[0] * 0.001f, msg.pose[1] * 0.001f, msg.pose[2]);
    player.ballPosition = Vector2f(msg.ball[0], msg.ball[1]) * 0.001f;
    static_assert(std::is_same_v<Clock::duration::period, std::chrono::seconds::period>);
    if (msg.ballAge < 0.f ||
        Clock::time_point{Clock::duration{msg.ballAge}} >= cycleInfo_->startTime)
    {
      player.timeWhenBallWasSeen = Clock::time_point{};
    }
    else
    {
      player.timeWhenBallWasSeen =
          cycleInfo_->startTime - std::chrono::duration_cast<Clock::duration>(
                                      std::chrono::duration<float, std::milli>(msg.ballAge));
    }
    player.fallen = (msg.fallen > 0);
    player.penalized = player.playerNumber <= rawGameControllerState_->penalties.size() &&
                       rawGameControllerState_->penalties[player.playerNumber - 1] != Penalty::NONE;

    // Parse the data array of the SPL standard message (DS msg and HULKs msg)
    auto bytesRead = parseDSMsg(msg, remainingBytes, message.receivedSystemTimePoint, player);
    remainingBytes = remainingBytes - bytesRead;
    // Check if parsing DS message failed
    if (bytesRead == 0)
    {
      Log<M_BRAIN>(LogLevel::WARNING)
          << "Unable to parse DevilSMASH message from player " << player.playerNumber;
    }
    else if (player.isHULK)
    {
      // Parse the hulks message
      // Note: In theory we are able to send a hulks message without a ds message. We might want to
      // move this out of this else case
      bytesRead = parseHULKMsg(msg, remainingBytes, player);
      remainingBytes = remainingBytes - bytesRead;
      // Check if parsing failed
      if (bytesRead == 0)
      {
        Log<M_BRAIN>(LogLevel::WARNING)
            << "Unable to parse HULKs message from player " << player.playerNumber;
      }
    }

    // Check if there is any data left OR if we read too much data
    if (remainingBytes != 0 && player.isHULK)
    {
      player.isHULK = false;
      player.currentlyPerformingRole = PlayingRole::DEFENDER;
      player.headYaw = 0;
      player.timeWhenReachBall = cycleInfo_->startTime + 12h;
      player.timeWhenReachBallStriker = cycleInfo_->startTime + 12h;
      player.lastTimeWhistleHeard = Clock::time_point{};
      player.currentPassTarget = -1;

      Log<M_BRAIN>(LogLevel::ERROR)
          << "Incoming SPL message data was not parsed correctly. Remaining number of bytes: "
          << remainingBytes;
    }

    bool merged = false;
    for (auto& it2 : internalPlayers_.rawPlayers)
    {
      if (it2.playerNumber == player.playerNumber)
      {
        it2 = player;
        merged = true;
        break;
      }
    }
    if (!merged)
    {
      internalPlayers_.rawPlayers.push_back(player);
      Log<M_BRAIN>(LogLevel::INFO)
          << "New Player " << player.playerNumber << " joined the network.";
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

unsigned int SPLMessageReceiver::parseDSMsg(
    const SPLStandardMessage::SPLStandardMessage& msg, unsigned int remainingBytes,
    [[maybe_unused]] const std::chrono::steady_clock::time_point& receivedSystemTimePoint,
    RawTeamPlayer& player)
{
  DevilSmash::StandardMessage devilSmashMsg;

  // Return if there is no data left
  if (remainingBytes == 0)
  {
    Log<M_BRAIN>(LogLevel::INFO)
        << "Received a SPL msg without DevilSMASH msg in custom data field";
    return 0;
  }

  // return if we cannot parse the header safely
  if (remainingBytes < sizeof(devilSmashMsg.header))
  {
    Log<M_BRAIN>(LogLevel::ERROR)
        << "RemainingBytes is smaller than size of DevilSMASH header. WTF";
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
    Log<M_BRAIN>(LogLevel::ERROR) << "sizeOfDSMessage > remaining SPL message bytes";
    return 0;
  }

  if (!devilSmashMsg.read(msg.data))
  {
    // invalidate data that may have been written to player.
    player.isHULK = false;
    player.currentlyPerformingRole = PlayingRole::DEFENDER;
    player.headYaw = 0;
    player.timeWhenReachBall = cycleInfo_->startTime + 12h;
    player.timeWhenReachBallStriker = cycleInfo_->startTime + 12h;
    player.lastTimeWhistleHeard = Clock::time_point{};
    player.currentPassTarget = -1;

    Log<M_BRAIN>(LogLevel::ERROR) << "Received a SPL msg with malformatted DevilSMASH msg";
    return 0;
  }

  assert(minimumSizeReached);

#ifdef HULK_TARGET_NAO
  if (devilSmashMsg.requestsNTPMessage)
  {
    NTPData::NTPRequest request;
    request.sender = msg.playerNum;
    request.origination = devilSmashMsg.timestamp;
    request.receipt = std::chrono::duration_cast<std::chrono::duration<unsigned int, std::milli>>(
                          receivedSystemTimePoint.time_since_epoch())
                          .count();
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
          static_cast<int>(
              ntpMsg.requestReceipt - ntpMsg.requestOrigination + devilSmashMsg.timestamp -
              std::chrono::duration_cast<std::chrono::duration<unsigned int, std::milli>>(
                  receivedSystemTimePoint.time_since_epoch())
                  .count()) /
          2;
      ntpRobots_[msg.playerNum - 1].valid = true;
    }
  }
#endif

  // figure out whether robot is a HULK
  player.isHULK = (devilSmashMsg.member == HULKS_MEMBER);
  // convert obstacle centers back to meters because the B-HULKs message is based on millimeters
  for (auto& playerObstacle : player.localObstacles)
  {
    playerObstacle.center[0] *= 0.001f;
    playerObstacle.center[1] *= 0.001f;
  }
  // override with info from BHULKs message because it might be that the GameController does not
  // know about a manually penalized robot
  player.penalized = devilSmashMsg.isPenalized;
  player.isPoseValid = devilSmashMsg.isRobotPoseValid;
  player.currentlyPerformingRole =
      DevilSmash::dsRoleToPlayingRole(devilSmashMsg.currentlyPerformingRole);
  player.roleAssignments.resize(DS_STANDARD_MESSAGE_MAX_NUM_PLAYERS);
  for (unsigned int i = 0; i < DS_STANDARD_MESSAGE_MAX_NUM_PLAYERS; i++)
  {
    player.roleAssignments[i] = DevilSmash::dsRoleToPlayingRole(devilSmashMsg.roleAssignments[i]);
  }
  player.ballVelocity =
      Vector2f(devilSmashMsg.ballVelocity[0] * 0.001f, devilSmashMsg.ballVelocity[1] * 0.001f);
  player.headYaw = devilSmashMsg.headYawAngle;
#ifdef HULK_TARGET_NAO
  if (ntpRobots_.size() >= static_cast<unsigned int>(msg.playerNum) &&
      ntpRobots_[msg.playerNum - 1].valid)
  {
    player.timeWhenReachBall = Clock::time_point{
        std::max(0ms, std::chrono::milliseconds{devilSmashMsg.timeWhenReachBall -
                                                ntpRobots_[msg.playerNum - 1].offset})};
    player.timeWhenReachBallStriker = Clock::time_point{
        std::max(0ms, std::chrono::milliseconds{devilSmashMsg.timeWhenReachBallStriker -
                                                ntpRobots_[msg.playerNum - 1].offset})};
    player.timestampLastJumped = Clock::time_point{
        std::max(0ms, std::chrono::milliseconds{devilSmashMsg.timestampLastJumped -
                                                ntpRobots_[msg.playerNum - 1].offset})};
    player.lastTimeWhistleHeard = Clock::time_point{
        std::max(0ms, std::chrono::milliseconds{devilSmashMsg.lastTimeWhistleDetected -
                                                ntpRobots_[msg.playerNum - 1].offset})};
    player.timeWhenBallWasSeen = Clock::time_point{
        std::max(0ms, std::chrono::milliseconds{devilSmashMsg.timeWhenBallLastSeen -
                                                ntpRobots_[msg.playerNum - 1].offset})};
  }
  else
  {
    player.timeWhenReachBall = cycleInfo_->startTime + 12h;
    player.timeWhenReachBallStriker = cycleInfo_->startTime + 12h;
    player.timestampLastJumped = Clock::time_point{};
    player.lastTimeWhistleHeard = Clock::time_point{};
    player.timeWhenBallWasSeen = Clock::time_point{};
  }
#else
  player.timeWhenReachBall =
      Clock::time_point{std::chrono::milliseconds{devilSmashMsg.timeWhenReachBall}};
  player.timeWhenReachBallStriker =
      Clock::time_point{std::chrono::milliseconds{devilSmashMsg.timeWhenReachBallStriker}};
  player.timestampLastJumped =
      Clock::time_point{std::chrono::milliseconds{devilSmashMsg.timestampLastJumped}};
  player.lastTimeWhistleHeard =
      Clock::time_point{std::chrono::milliseconds{devilSmashMsg.lastTimeWhistleDetected}};
  player.timeWhenBallWasSeen =
      Clock::time_point{std::chrono::milliseconds{devilSmashMsg.timeWhenBallLastSeen}};
#endif

  return devilSmashMsg.sizeOfDSMessage();
}

unsigned int SPLMessageReceiver::parseHULKMsg(const SPLStandardMessage::SPLStandardMessage& msg,
                                              unsigned int remainingBytes, RawTeamPlayer& player)
{
  HULKs::HULKsMessage hulksMessage;

  // return if there is no data left
  if (remainingBytes == 0)
  {
    Log<M_BRAIN>(LogLevel::INFO) << "Received a SPL msg without HULKs msg in custom data field";
    return 0;
  }

  // return if we cannot parse the header safely
  if (remainingBytes < sizeof(hulksMessage.header))
  {
    Log<M_BRAIN>(LogLevel::ERROR) << "RemainingBytes is smaller than size of HULKs header. WTF";
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
    Log<M_BRAIN>(LogLevel::ERROR) << "sizeOfHULKsMessage > remaining SPL message bytes";
    return 0;
  }

  if (!hulksMessage.read(msg.data + (msg.numOfDataBytes - remainingBytes)))
  {
    Log<M_BRAIN>(LogLevel::ERROR) << "Received a SPL msg with malformatted HULKs msg";
    return 0;
  }

  assert(minimumSizeReached);

  player.walkingTo = hulksMessage.walkingTo;
  player.currentPassTarget = hulksMessage.passTarget;
  // add local obstacles of the robot to the RawTeamPlayer
  player.localObstacles = hulksMessage.obstacles;
  for (auto& obstacle : player.localObstacles)
  {
    obstacle.center[0] *= 0.001f;
    obstacle.center[1] *= 0.001f;
  }
  player.currentSearchPosition = hulksMessage.ballSearchData.currentSearchPosition;

  for (uint8_t i = 0; i < MAX_NUM_PLAYERS; i++)
  {
    player.suggestedSearchPositionsValidity[i] =
        static_cast<bool>(hulksMessage.ballSearchData.positionSuggestionsValidity & (1u << i));
    player.suggestedSearchPositions[i] = hulksMessage.ballSearchData.positionSuggestions[i];
  }

#ifdef HULK_TARGET_NAO
  if (ntpRobots_.size() >= static_cast<unsigned int>(msg.playerNum) &&
      ntpRobots_[msg.playerNum - 1].valid)
  {
    player.timestampBallSearchMapUnreliable = Clock::time_point{std::max(
        0ms,
        std::chrono::milliseconds{hulksMessage.ballSearchData.timestampBallSearchMapUnreliable -
                                  ntpRobots_[msg.playerNum - 1].offset})};
  }
#else
  player.timestampBallSearchMapUnreliable = Clock::time_point{
      std::chrono::milliseconds{hulksMessage.ballSearchData.timestampBallSearchMapUnreliable}};
#endif

  player.mostWisePlayerNumber = hulksMessage.ballSearchData.mostWisePlayerNumber;

  return hulksMessage.sizeOfHULKsMessage();
}
