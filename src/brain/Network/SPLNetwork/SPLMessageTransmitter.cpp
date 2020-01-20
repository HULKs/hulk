#include <cstring>

#include "Definitions/DevilSmashStandardMessage.hpp"
#include "Definitions/SPLStandardMessage.h"

#include "DSHelper.hpp"
#include "HULKsMessage.hpp"
#include "HULKsMessageHelper.hpp"
#include "SPLMessageTransmitter.hpp"


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
  , motionRequest_(*this)
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

void SPLMessageTransmitter::cycle()
{
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

  // check if message sending is allowed
  if (cycleInfo_->getTimeDiff(lastTime_) < 1.f / msgPerSecond_())
  {
    return;
  }
  // only transmit via wifi if configured so.
  if (networkServiceData_->valid && !transmitViaEthernet_() &&
      !networkServiceData_->isConnectedToAnyWifi)
  {
    return;
  }

  SPLStandardMessage msg;
  msg.playerNum = static_cast<uint8_t>(playerConfiguration_->playerNumber);
  msg.teamNum = static_cast<uint8_t>(playerConfiguration_->teamNumber);
  msg.fallen = static_cast<uint8_t>(bodyPose_->fallen);

  msg.pose[0] = robotPosition_->pose.position.x() * 1000.f;
  msg.pose[1] = robotPosition_->pose.position.y() * 1000.f;
  msg.pose[2] = robotPosition_->pose.orientation;

  if (ballState_->confident)
  {
    msg.ballAge = ballState_->age;
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
  devilSmashMsg.headYawAngle = jointSensorData_->angles[JOINTS::HEAD_YAW];
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

  // The default initialization of both times is a timepoint that is as far in the future as
  // possible.
  if (timeToReachBall_->valid)
  {
    devilSmashMsg.timeWhenReachBall =
        cycleInfo_->startTime.getSystemTime() +
        static_cast<uint32_t>(timeToReachBall_->timeToReachBall * 1000);
    devilSmashMsg.timeWhenReachBallStriker =
        cycleInfo_->startTime.getSystemTime() +
        static_cast<uint32_t>(timeToReachBall_->timeToReachBallStriker * 1000);
  }
  devilSmashMsg.timeWhenBallLastSeen = ballState_->timeWhenLastSeen.getSystemTime();
  devilSmashMsg.ballVelocity[0] = ballState_->velocity.x() * 1000;
  devilSmashMsg.ballVelocity[1] = ballState_->velocity.y() * 1000;
  devilSmashMsg.ballValidity = ballState_->validity;
  devilSmashMsg.timestampLastJumped = robotPosition_->lastTimeJumped.getSystemTime();
  devilSmashMsg.lastTimeWhistleDetected = whistleData_->lastTimeWhistleHeard.getSystemTime();

  if (cycleInfo_->getTimeDiff(lastNTPRequest_) > 2.0f)
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
  devilSmashMsg.timestamp = TimePoint::getCurrentTime().getSystemTime();
  if (devilSmashMsg.sizeOfDSMessage() <= SPL_STANDARD_MESSAGE_DATA_SIZE)
  {
    devilSmashMsg.write(msg.data);
    msg.numOfDataBytes = devilSmashMsg.sizeOfDSMessage();

    HULKs::HULKsMessage hulksmsg;

    if (motionRequest_->bodyMotion == MotionRequest::BodyMotion::WALK)
    {
      hulksmsg.walkingTo = robotPosition_->robotToField(motionRequest_->walkData.target);
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

    for (auto& obstacle : obstacleData_->obstacles)
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
      hulksObstacle.timestampLastSeen = cycleInfo_->startTime.getSystemTime();
      hulksObstacle.type = HULKs::obstacleTypeToHMObstacleType(obstacle.type);
      assert(hulksObstacle.type != HULKs::ObstacleType::MAX);
      hulksmsg.obstacles.push_back(hulksObstacle);
    }

    HULKs::BallSearchData& ballSearchData = hulksmsg.ballSearchData;

    ballSearchData.currentSearchPosition = ballSearchPosition_->searchPosition;
    ballSearchData.availableForSearch = ballSearchPosition_->availableForSearch;
    // std::cout << static_cast<int>(msg.playerNum) << "T is available for search " <<
    // (ballSearchData.availableForSearch ? "True" : "False") << std::endl;

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
        ballSearchMap_->timestampBallSearchMapUnreliable_.getSystemTime();
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
