#include <cstring>

#include "Definitions/BHULKsStandardMessage.h"
#include "Definitions/SPLStandardMessage.h"

#include "BHULKsHelper.hpp"
#include "HULKsMessage.hpp"
#include "SPLMessageTransmitter.hpp"


SPLMessageTransmitter::SPLMessageTransmitter(const ModuleManagerInterface& manager)
  : Module(manager)
  , sendSonarObstacles_(*this, "sendSonarObstacles", [] {})
  , playerConfiguration_(*this)
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
  bufferedNTPRequests_.insert(bufferedNTPRequests_.begin(), ntpData_->ntpRequests.begin(),
                              ntpData_->ntpRequests.end());
  // check if message sending is allowed
  if (cycleInfo_->getTimeDiff(lastTime_) < 0.34f)
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

  B_HULKs::BHULKsStandardMessage bhmsg;
  bhmsg.member = HULKS_MEMBER;
  bhmsg.isUpright = !bodyPose_->fallen;
  bhmsg.hasGroundContact = bodyPose_->footContact;
  bhmsg.timeOfLastGroundContact = bodyPose_->timeOfLastFootContact.getSystemTime();
  B_HULKs::OwnTeamInfo ownTeamInfo;
  ownTeamInfo.timestampWhenReceived = gameControllerState_->timestampOfLastMessage.getSystemTime();
  ownTeamInfo.packetNumber = gameControllerState_->packetNumber;
  ownTeamInfo.state.fromGameControllerState(*gameControllerState_);
  ownTeamInfo.kickingTeam = gameControllerState_->kickingTeamNumber;
  ownTeamInfo.dropInTeam = gameControllerState_->dropInTeam;
  ownTeamInfo.dropInTime = gameControllerState_->dropInTime;
  ownTeamInfo.secsRemaining = gameControllerState_->remainingTime;
  ownTeamInfo.secondaryTime = gameControllerState_->secondaryTime;
  ownTeamInfo.score = gameControllerState_->score;
  for (unsigned int i = 0; i < BHULKS_STANDARD_MESSAGE_MAX_NUM_OF_PLAYERS; i++)
  {
    ownTeamInfo.playersArePenalized[i] = (i < gameControllerState_->penalties.size())
                                             ? (gameControllerState_->penalties[i] != Penalty::NONE)
                                             : false;
  }
  bhmsg.isPenalized = (gameControllerState_->penalty != Penalty::NONE) ||
                      (gameControllerState_->gameState == GameState::INITIAL &&
                       !gameControllerState_->chestButtonWasPressedInInitial);
  bhmsg.headYawAngle = jointSensorData_->angles[JOINTS::HEAD_YAW];
  bhmsg.currentlyPerfomingRole = B_HULKs::playingToBHULKRole(playingRoles_->role);
  for (unsigned int i = 0; i < BHULKS_STANDARD_MESSAGE_MAX_NUM_OF_PLAYERS; i++)
  {
    if (i < playingRoles_->playerRoles.size())
    {
      bhmsg.roleAssignments[i] = B_HULKs::playingToBHULKRole(playingRoles_->playerRoles[i]);
    }
    else
    {
      bhmsg.roleAssignments[i] = B_HULKs::Role::beatenPieces;
    }
  }

  if (playingRoles_->role == PlayingRole::KEEPER && keeperAction_->action.valid &&
      keeperAction_->wantsToPlayBall)
  {
    bhmsg.kingIsPlayingBall = true;
  }

  if (playingRoles_->role == PlayingRole::STRIKER && strikerAction_->valid &&
      strikerAction_->type == StrikerAction::PASS)
  {
    bhmsg.passTarget = strikerAction_->passTarget;
  }
  // The default initialization of both times is a timepoint that is as far in the future as
  // possible.
  if (timeToReachBall_->valid)
  {
    bhmsg.timeWhenReachBall =
        cycleInfo_->startTime.getSystemTime() + timeToReachBall_->timeToReachBall * 1000;
    bhmsg.timeWhenReachBallQueen =
        cycleInfo_->startTime.getSystemTime() + timeToReachBall_->timeToReachBallStriker * 1000;
  }
  bhmsg.ballTimeWhenLastSeen = ballState_->timeWhenLastSeen.getSystemTime();
  bhmsg.timestampLastJumped = robotPosition_->lastTimeJumped.getSystemTime();
  bhmsg.confidenceOfLastWhistleDetection = B_HULKs::HearingConfidence::allEarsAreOk;
  bhmsg.lastTimeWhistleDetected = whistleData_->lastTimeWhistleHeard.getSystemTime();
  for (auto& obstacle : obstacleData_->obstacles)
  {
    // there can not be INVALID obstacles at this stage anymore
    assert(obstacle.type != ObstacleType::INVALID);
    // TODO: Refactor sonar stuff to unknown
    if (obstacle.type == ObstacleType::BALL ||
        (obstacle.type == ObstacleType::UNKNOWN && !sendSonarObstacles_()))
    {
      continue;
    }
    B_HULKs::Obstacle bhObstacle;
    bhObstacle.center[0] = obstacle.relativePosition.x() * 1000.f;
    bhObstacle.center[1] = obstacle.relativePosition.y() * 1000.f;
    bhObstacle.timestampLastSeen = cycleInfo_->startTime.getSystemTime();
    bhObstacle.type = static_cast<B_HULKs::ObstacleType>(obstacle.type);
    bhmsg.obstacles.push_back(bhObstacle);
  }
  if (cycleInfo_->getTimeDiff(lastNTPRequest_) > 2.0f)
  {
    bhmsg.requestsNTPMessage = true;
    lastNTPRequest_ = cycleInfo_->startTime;
  }
  bhmsg.ntpMessages.reserve(bufferedNTPRequests_.size());
  for (auto& ntpRequest : bufferedNTPRequests_)
  {
    B_HULKs::BNTPMessage ntpMessage;
    ntpMessage.receiver = ntpRequest.sender;
    ntpMessage.requestOrigination = ntpRequest.origination;
    ntpMessage.requestReceipt = ntpRequest.receipt;
    bhmsg.ntpMessages.push_back(ntpMessage);
  }
  // The list is cleared even if the BH message is not sent because otherwise the message could
  // never be sent.
  bufferedNTPRequests_.clear();
  // This is the last possible time point to set the time of the message.
  // Use getCurrentTime here, because it is better for NTP.
  bhmsg.timestamp = TimePoint::getCurrentTime().getSystemTime();
  if (bhmsg.sizeOfBHULKsMessage() <= SPL_STANDARD_MESSAGE_DATA_SIZE)
  {
    bhmsg.write(msg.data);
    msg.numOfDataBytes = bhmsg.sizeOfBHULKsMessage();

    HULKs::HULKsMessage hulksmsg;
    hulksmsg.isPoseValid = robotPosition_->valid;

    if (motionRequest_->bodyMotion == MotionRequest::BodyMotion::WALK)
    {
      hulksmsg.walkingTo = robotPosition_->robotToField(motionRequest_->walkData.target);
    }
    else
    {
      hulksmsg.walkingTo = robotPosition_->pose;
    }

    hulksmsg.ballVel[0] = ballState_->velocity.x();
    hulksmsg.ballVel[1] = ballState_->velocity.y();

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
