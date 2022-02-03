#include "Brain/Network/SPLNetwork/SPLMessageTransmitter.hpp"
#include "Brain/Network/SPLNetwork/DSHelper.hpp"
#include "Brain/Network/SPLNetwork/HULKsMessage.hpp"
#include "Brain/Network/SPLNetwork/HULKsMessageHelper.hpp"
#include <chrono>
#include <cstring>
#include <type_traits>

SPLMessageTransmitter::SPLMessageTransmitter(const ModuleManagerInterface& manager)
  : Module(manager)
  , msgPerSecond_(*this, "msgPerSecond", [] {})
  , transmitViaEthernet_(*this, "transmitViaEthernet", [] {})
  , sendSonarObstacles_(*this, "sendSonarObstacles", [] {})
  , fakeMemberFlag_(*this, "fakeMemberFlag", [] {})
  , playerConfiguration_(*this)
  , networkServiceData_(*this)
  , ballState_(*this)
  , robotPosition_(*this)
  , bodyPose_(*this)
  , splNetworkData_(*this)
  , playingRoles_(*this)
  , actionCommand_(*this)
  , ntpData_(*this)
  , strikerAction_(*this)
  , keeperAction_(*this)
  , whistleData_(*this)
  , timeToReachBall_(*this)
  , cycleInfo_(*this)
  , gameControllerState_(*this)
  , obstacleData_(*this)
  , jointSensorData_(*this)
  , teamBallModel_(*this)
  , ballSearchMap_(*this)
  , ballSearchPosition_(*this)
{
}

// NOLINTNEXTLINE(readability-function-cognitive-complexity)
void SPLMessageTransmitter::cycle()
{
#ifdef HULK_TARGET_NAO
  // copy NTP requests to local buffer in any case
  for (const auto& newRequest : ntpData_->ntpRequests)
  {
    bool found = false;
    for (auto& bufferedRequest : bufferedNTPRequests_)
    {
      if (bufferedRequest.sender == newRequest.sender)
      {
        bufferedRequest = newRequest;
        found = true;
      }
    }

    if (!found)
    {
      bufferedNTPRequests_.push_back(newRequest);
    }
  }
  assert(bufferedNTPRequests_.size() <= 6);
#endif

  // check if message sending is allowed
  if (cycleInfo_->getAbsoluteTimeDifference(lastTime_) < 1s / msgPerSecond_())
  {
    return;
  }
  // only transmit via wifi if configured so.
  if (networkServiceData_->valid && !transmitViaEthernet_() &&
      !networkServiceData_->isConnectedToAnyWifi)
  {
    return;
  }

  SPLStandardMessage::SPLStandardMessage msg;
  msg.playerNum = static_cast<uint8_t>(playerConfiguration_->playerNumber);
  msg.teamNum = static_cast<uint8_t>(playerConfiguration_->teamNumber);
  msg.fallen = static_cast<uint8_t>(bodyPose_->fallen);

  msg.pose[0] = robotPosition_->pose.x() * 1000.f;
  msg.pose[1] = robotPosition_->pose.y() * 1000.f;
  msg.pose[2] = robotPosition_->pose.angle();

  if (ballState_->confident)
  {
    static_assert(std::is_same_v<Clock::duration::period, std::chrono::seconds::period>);
    msg.ballAge = ballState_->age.count();
  }
  else
  {
    msg.ballAge = 1337.f;
  }
  msg.ball[0] = ballState_->position.x() * 1000.f;
  msg.ball[1] = ballState_->position.y() * 1000.f;

  DevilSmash::StandardMessage devilSmashMsg;
  devilSmashMsg.member = fakeMemberFlag_() ? DEVIL_MEMBER : HULKS_MEMBER;
  devilSmashMsg.isPenalized = (gameControllerState_->penalty != Penalty::NONE) ||
                              (gameControllerState_->gameState == GameState::INITIAL &&
                               !gameControllerState_->chestButtonWasPressedInInitial);
  devilSmashMsg.isRobotPoseValid = robotPosition_->valid;
  devilSmashMsg.headYawAngle = jointSensorData_->angles[Joints::HEAD_YAW];
  devilSmashMsg.currentlyPerformingRole = DevilSmash::playingToDSRole(playingRoles_->role);
  for (unsigned int i = 0; i < DS_STANDARD_MESSAGE_MAX_NUM_PLAYERS; i++)
  {
    if (i < playingRoles_->playerRoles.size())
    {
      devilSmashMsg.roleAssignments[i] = DevilSmash::playingToDSRole(playingRoles_->playerRoles[i]);
    }
    else
    {
      devilSmashMsg.roleAssignments[i] = DevilSmash::Role::MAX;
    }
  }

  // The default initialization of both times is a Clock::time_point that is as far in the future as
  // possible.
  if (timeToReachBall_->valid)
  {
    devilSmashMsg.timeWhenReachBall =
        std::chrono::duration_cast<std::chrono::milliseconds>(
            (cycleInfo_->startTime + timeToReachBall_->timeToReachBall).time_since_epoch())
            .count();
    devilSmashMsg.timeWhenReachBallStriker =
        std::chrono::duration_cast<std::chrono::milliseconds>(
            (cycleInfo_->startTime + timeToReachBall_->timeToReachBallStriker).time_since_epoch())
            .count();
  }
  devilSmashMsg.timeWhenBallLastSeen = std::chrono::duration_cast<std::chrono::milliseconds>(
                                           ballState_->timeWhenLastSeen.time_since_epoch())
                                           .count();
  devilSmashMsg.ballVelocity[0] = ballState_->velocity.x() * 1000;
  devilSmashMsg.ballVelocity[1] = ballState_->velocity.y() * 1000;
  devilSmashMsg.ballValidity = ballState_->validity;
  devilSmashMsg.timestampLastJumped = std::chrono::duration_cast<std::chrono::milliseconds>(
                                          robotPosition_->lastTimeJumped.time_since_epoch())
                                          .count();
  devilSmashMsg.lastTimeWhistleDetected = std::chrono::duration_cast<std::chrono::milliseconds>(
                                              whistleData_->lastTimeWhistleHeard.time_since_epoch())
                                              .count();

#ifdef HULK_TARGET_NAO
  if (cycleInfo_->getAbsoluteTimeDifference(lastNTPRequest_) > 2s)
  {
    devilSmashMsg.requestsNTPMessage = true;
    lastNTPRequest_ = cycleInfo_->startTime;
  }
  devilSmashMsg.ntpMessages.reserve(bufferedNTPRequests_.size());
  for (auto& ntpRequest : bufferedNTPRequests_)
  {
    DevilSmash::NTPMessage ntpMessage;
    ntpMessage.receiver = ntpRequest.sender;
    ntpMessage.requestOrigination = ntpRequest.origination;
    ntpMessage.requestReceipt = ntpRequest.receipt;
    devilSmashMsg.ntpMessages.push_back(ntpMessage);
  }
  // The list is cleared even if the BH message is not sent because otherwise the message could
  // never be sent.
  bufferedNTPRequests_.clear();
  // This is the last possible time point to set the time of the message.
  // Use getCurrentTime here, because it is better for NTP.
  devilSmashMsg.timestamp = std::chrono::duration_cast<std::chrono::milliseconds>(
                                std::chrono::steady_clock::now().time_since_epoch())
                                .count();
#else
  // Use CycleInfo for timestamp
  devilSmashMsg.timestamp = std::chrono::duration_cast<std::chrono::milliseconds>(
                                cycleInfo_->startTime.time_since_epoch())
                                .count();
#endif
  if (devilSmashMsg.sizeOfDSMessage() <= SPL_STANDARD_MESSAGE_DATA_SIZE)
  {
    devilSmashMsg.write(msg.data);
    msg.numOfDataBytes = devilSmashMsg.sizeOfDSMessage();

    HULKs::HULKsMessage hulksmsg;

    if (actionCommand_->body().type == ActionCommand::Body::MotionType::WALK)
    {
      hulksmsg.walkingTo = robotPosition_->robotToField(actionCommand_->body().walkTarget);
    }
    else
    {
      hulksmsg.walkingTo = robotPosition_->pose;
    }

    if (playingRoles_->role == PlayingRole::STRIKER && strikerAction_->valid &&
        strikerAction_->type == StrikerAction::Type::PASS)
    {
      hulksmsg.passTarget = strikerAction_->passTarget;
    }

    for (const auto& obstacle : obstacleData_->obstacles)
    {
      // there can not be INVALID obstacles at this stage anymore
      assert(obstacle.type != ObstacleType::INVALID);
      assert(obstacle.type != ObstacleType::OBSTACLETYPE_MAX);
      // TODO: Refactor sonar stuff to unknown
      // Don't send free kick area obstacles
      if (obstacle.type == ObstacleType::BALL || obstacle.type == ObstacleType::FREE_KICK_AREA ||
          obstacle.type == ObstacleType::GOAL_POST ||
          (obstacle.type == ObstacleType::UNKNOWN && !sendSonarObstacles_()))
      {
        continue;
      }
      HULKs::Obstacle hulksObstacle;
      hulksObstacle.center[0] = obstacle.relativePosition.x() * 1000.f;
      hulksObstacle.center[1] = obstacle.relativePosition.y() * 1000.f;
      hulksObstacle.timestampLastSeen = std::chrono::duration_cast<std::chrono::milliseconds>(
                                            cycleInfo_->startTime.time_since_epoch())
                                            .count();
      hulksObstacle.type = HULKs::obstacleTypeToHMObstacleType(obstacle.type);
      assert(hulksObstacle.type != HULKs::ObstacleType::MAX);
      hulksmsg.obstacles.push_back(hulksObstacle);
    }

    HULKs::BallSearchData& ballSearchData = hulksmsg.ballSearchData;

    ballSearchData.currentSearchPosition = ballSearchPosition_->searchPosition;

    assert(ballSearchPosition_->suggestedSearchPositionValid.size() == MAX_NUM_PLAYERS &&
           "suggestion valid flag array size mismatch");
    ballSearchData.positionSuggestionsValidity = 0;
    // Set the valid bit for every position suggestion.
    for (uint8_t i = 0; i < MAX_NUM_PLAYERS; i++)
    {
      ballSearchData.positionSuggestionsValidity |=
          ballSearchPosition_->suggestedSearchPositionValid[i] << i;
    }

    ballSearchData.positionSuggestions.resize(MAX_NUM_PLAYERS);
    for (unsigned int i = 0; i < ballSearchPosition_->suggestedSearchPositions.size(); i++)
    {
      ballSearchData.positionSuggestions[i] = ballSearchPosition_->suggestedSearchPositions[i];
    }

    ballSearchData.timestampBallSearchMapUnreliable =
        std::chrono::duration_cast<std::chrono::milliseconds>(
            ballSearchMap_->timestampBallSearchMapUnreliable.time_since_epoch())
            .count();
    ballSearchData.mostWisePlayerNumber = ballSearchPosition_->localMostWisePlayerNumber;

    if (msg.numOfDataBytes + hulksmsg.sizeOfHULKsMessage() <= SPL_STANDARD_MESSAGE_DATA_SIZE)
    {
      hulksmsg.write(msg.data + msg.numOfDataBytes);
      msg.numOfDataBytes += hulksmsg.sizeOfHULKsMessage();
    }
  }

  // send the message asynchronously via the SPLNetworkService
  splNetworkData_->sendMessage(msg);
  lastTime_ = cycleInfo_->startTime;
}
