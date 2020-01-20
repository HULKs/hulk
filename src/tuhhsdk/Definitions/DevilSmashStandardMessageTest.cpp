#include "DevilSmashStandardMessageTest.hpp"

#include "SPLStandardMessage.h"

#include <random>

namespace DevilSmash
{
  constexpr float pi = 3.1415926535897932384626433832795f;

  bool DevilSmash::DevilSmashStandardMessageTest::test()
  {
    uint8_t data[SPL_STANDARD_MESSAGE_DATA_SIZE];

    StandardMessage origMsg;

    origMsg.member = HULKS_MEMBER;
    origMsg.timestamp = randomInt(0xFFFFu, 0xFFFFFFu);
    origMsg.isPenalized = randomBool();
    origMsg.headYawAngle = randomBool() ? 0.5f : -0.5f;
    origMsg.currentlyPerformingRole = Role::STRIKER;
    for (unsigned int player = 0; player < DS_STANDARD_MESSAGE_MAX_NUM_PLAYERS; player++)
    {
      origMsg.roleAssignments[player] = randomBool() ? static_cast<Role>(player) : Role::DEFENDER_LEFT;
    }

    origMsg.gameState.setPlay = randomInt(0, 7);
    origMsg.gameState.gameState = randomInt(0, 7);
    origMsg.gameState.gamePhase = randomInt(0, 1);
    origMsg.gameState.competitionType = randomInt(0, 3);
    origMsg.gameState.competitionPhase = randomInt(0, 1);
    origMsg.gameState.firstHalf = randomBool();
    origMsg.gameState.kickingTeam = randomBool();

    origMsg.timeWhenReachBall = randomInt(origMsg.timestamp, origMsg.timestamp + 120);
    origMsg.timeWhenReachBallStriker = randomInt(origMsg.timestamp, origMsg.timestamp + 110);
    origMsg.timeWhenBallLastSeen = randomInt(0, origMsg.timestamp);
    origMsg.timestampLastJumped = randomInt(0, origMsg.timestamp);
    origMsg.lastTimeWhistleDetected = randomInt(origMsg.timestamp - 0xFFFF, origMsg.timestamp);

    origMsg.ballVelocity[0] = static_cast<float>(randomInt(0, 10000)) - 5000.f;
    origMsg.ballVelocity[1] = static_cast<float>(randomInt(0, 10000)) - 5000.f;

    const unsigned int numRobots = randomInt(0, DS_STANDARD_MESSAGE_MAX_ROBOTS_IN_MAP);
    for (unsigned int i = 0; i < numRobots; i++)
    {
      RobotMap::Robot robot;
      robot.type = static_cast<RobotMap::Robot::Type>(randomInt(0, 2));
      robot.x = static_cast<float>(static_cast<int32_t>(randomInt(0, 12000)) - 6000);
      robot.y = static_cast<float>(static_cast<int32_t>(randomInt(0, 12000)) - 6000);

      origMsg.robotMap.map.emplace_back(robot);
    }

    origMsg.requestsNTPMessage = randomBool();

    /*for (unsigned int player = 0; player < DS_STANDARD_MESSAGE_MAX_NUM_PLAYERS; player++)
    {
      if (randomBool())
      {
        NTPMessage ntpMsg;
        ntpMsg.requestOrigination = randomInt();
        ntpMsg.requestReceipt = randomInt(0, 4000);
        ntpMsg.receiver = player;
        origMsg.ntpMessages.emplace_back(ntpMsg);
      }
    }*/

    origMsg.write(data);


    StandardMessage readMsg;
    readMsg.read(data);

    checkEqual<decltype(origMsg.version)>(origMsg.version, readMsg.version);
    checkEqual<decltype(origMsg.timestamp)>(origMsg.timestamp, readMsg.timestamp);
    checkEqual<decltype(origMsg.headYawAngle)>(origMsg.headYawAngle, readMsg.headYawAngle, 1.1f / 180.f * pi);
    if (origMsg.timestamp - origMsg.timestampLastJumped <= (250u << 7u))
      checkEqual<decltype(origMsg.timestampLastJumped)>(origMsg.timestampLastJumped, readMsg.timestampLastJumped, 129);
    checkEqual<decltype(origMsg.timeWhenReachBall)>(origMsg.timeWhenReachBall, readMsg.timeWhenReachBall, 9);
    checkEqual<decltype(origMsg.timeWhenReachBallStriker)>(origMsg.timeWhenReachBallStriker, readMsg.timeWhenReachBallStriker, 9);
    checkEqual<decltype(origMsg.timeWhenBallLastSeen)>(origMsg.timeWhenBallLastSeen, readMsg.timeWhenBallLastSeen);
    checkEqual<decltype(origMsg.ballVelocity[0])>(origMsg.ballVelocity[0], readMsg.ballVelocity[0], 1.f);
    checkEqual<decltype(origMsg.ballVelocity[1])>(origMsg.ballVelocity[1], readMsg.ballVelocity[1], 1.f);
    checkEqual<decltype(origMsg.lastTimeWhistleDetected)>(origMsg.lastTimeWhistleDetected, readMsg.lastTimeWhistleDetected);

    checkEqual<decltype(origMsg.gameState.setPlay)>(origMsg.gameState.setPlay, readMsg.gameState.setPlay);
    checkEqual<decltype(origMsg.gameState.gameState)>(origMsg.gameState.gameState, readMsg.gameState.gameState);
    checkEqual<decltype(origMsg.gameState.gamePhase)>(origMsg.gameState.gamePhase, readMsg.gameState.gamePhase);
    checkEqual<decltype(origMsg.gameState.competitionType)>(origMsg.gameState.competitionType, readMsg.gameState.competitionType);
    checkEqual<decltype(origMsg.gameState.competitionPhase)>(origMsg.gameState.competitionPhase, readMsg.gameState.competitionPhase);
    checkEqual<decltype(origMsg.gameState.firstHalf)>(origMsg.gameState.firstHalf, readMsg.gameState.firstHalf);
    checkEqual<decltype(origMsg.gameState.kickingTeam)>(origMsg.gameState.kickingTeam, readMsg.gameState.kickingTeam);

    checkEqual<uint8_t>(static_cast<uint8_t>(origMsg.currentlyPerformingRole),
                        static_cast<uint8_t>(readMsg.currentlyPerformingRole));
    for (unsigned int player = 0; player < DS_STANDARD_MESSAGE_MAX_NUM_PLAYERS; player++)
    {
      checkEqual<uint8_t>(static_cast<uint8_t>(origMsg.roleAssignments[player]),
                          static_cast<uint8_t>(readMsg.roleAssignments[player]));
    }

    checkEqual<decltype(origMsg.member)>(origMsg.member, readMsg.member);
    checkEqual<decltype(origMsg.isPenalized)>(origMsg.isPenalized, readMsg.isPenalized);
    checkEqual<decltype(origMsg.requestsNTPMessage)>(origMsg.requestsNTPMessage, readMsg.requestsNTPMessage);

    checkEqual<decltype(origMsg.robotMap.map.size())>(origMsg.robotMap.map.size(), readMsg.robotMap.map.size());
    for (unsigned int i = 0; i < readMsg.robotMap.map.size(); i++)
    {
      checkEqual<uint8_t>(static_cast<uint8_t>(origMsg.robotMap.map[i].type), static_cast<uint8_t>(readMsg.robotMap.map[i].type));
      checkEqual<float>(origMsg.robotMap.map[i].x, readMsg.robotMap.map[i].x, 0.01);
      checkEqual<float>(origMsg.robotMap.map[i].y, readMsg.robotMap.map[i].y, 0.01);
    }

    checkEqual<decltype(origMsg.ntpMessages.size())>(origMsg.ntpMessages.size(), readMsg.ntpMessages.size());

    return true;
  }

  bool DevilSmashStandardMessageTest::randomBool()
  {
    return randomInt(0, 1) > 0;
  }

  uint32_t DevilSmashStandardMessageTest::randomInt(uint32_t min, uint32_t max)
  {
    assert(min < max);

    std::random_device r;
    std::default_random_engine e1(r());
    std::uniform_int_distribution<uint32_t> uniformDist(min, max);
    uint32_t randomNumber = uniformDist(e1);
    assert(randomNumber >= min && randomNumber <= max);
    return randomNumber;
  }

} // namespace DevilSmash
