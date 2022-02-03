#include "Messages/DevilSmashStandardMessage.hpp"

#include <cstring>

namespace DevilSmash
{
  /// Some weird math constant
  constexpr float pi = 3.1415926535897932384626433832795f;

  /**
   * @brief writeVal writes a given value into the given data field and increases the data pointer
   * accordingly
   * @tparam T the type of the value to write
   * @param data the target data field
   * @param value the actual value to write into data
   */
  template <typename T>
  inline void writeVal(void*& data, T value)
  {
    *reinterpret_cast<T*>(data) = value;
    reinterpret_cast<char*&>(data) += sizeof(T);
  }

  /**
   * @brief readVal reads a value from the given data field and increases the data pointer
   * accordingly
   * @tparam T the type of the value to read
   * @param data the data field to read the value from
   * @return the value that has been read from the data field
   */
  template <typename T>
  inline T readVal(const void*& data)
  {
    const T val = *reinterpret_cast<const T*>(data);
    reinterpret_cast<const char*&>(data) += sizeof(T);
    return val;
  }

  /**
   * @brief shiftAndClip shifts the given value "shift" times to the right and returns fallbackValue
   * iff value > maxVal
   * @tparam T the type of the given value
   * @param value the value to shift and clip
   * @param maxVal the maximum value after shifting
   * @param fallbackValue the value to return if value > maxVal after shifting
   * @param shift the amount of shifts to the right.
   * @return shifted and clipped value
   */
  template <typename T>
  inline T shiftAndClip(T value, uint64_t maxVal, uint64_t fallbackValue, uint8_t shift)
  {
    value >>= shift;
    value = value > static_cast<T>(maxVal) ? static_cast<T>(fallbackValue) : value;
    return value;
  }

  int RobotMap::sizeOf() const
  {
    // since the robot type will be serialized into 2 bits per robot (2 * 16 = 32)
    static_assert(DS_STANDARD_MESSAGE_MAX_ROBOTS_IN_MAP <= 16,
                  "Robot map is not able to handle this many robots");

    return 1                 // Number of robots in map
           + 4               // Robot type container (2 bit per robot in map)
           + 4 * map.size(); // Robot coordinates as int16_t
  }

  void RobotMap::write(void*& data) const
  {
    const unsigned int robotsInMap =
        std::min(static_cast<int>(map.size()), DS_STANDARD_MESSAGE_MAX_ROBOTS_IN_MAP);
    writeVal<uint8_t>(data, static_cast<uint8_t>(robotsInMap));

    uint32_t robotTypeContainer = 0;
    for (unsigned int i = 0; i < robotsInMap; i++)
    {
      assert(map[i].type != Robot::Type::MAX);

      writeVal<int16_t>(data, static_cast<int16_t>(map[i].x * 4.f));
      writeVal<int16_t>(data, static_cast<int16_t>(map[i].y * 4.f));
      (robotTypeContainer <<= 2u) |= static_cast<uint8_t>(map[i].type);
    }
    writeVal<uint32_t>(data, robotTypeContainer);
  }

  void RobotMap::read(const void*& data)
  {
    map.clear();

    const auto robotsInMap = readVal<uint8_t>(data);

    for (unsigned int i = 0; i < robotsInMap; i++)
    {
      Robot robot;
      robot.x = static_cast<float>(readVal<int16_t>(data)) / 4.f;
      robot.y = static_cast<float>(readVal<int16_t>(data)) / 4.f;
      robot.type = Robot::Type::MAX;
      map.emplace_back(robot);
    }

    auto robotTypeContainer = readVal<uint32_t>(data);
    for (auto it = map.rbegin(); it != map.rend(); it++)
    {
      it->type = static_cast<Robot::Type>(robotTypeContainer & 3u);
      assert(it->type != Robot::Type::MAX);
      robotTypeContainer >>= 2u;
    }
  }


  void GameStateStruct::write(void*& data) const
  {
    const uint16_t setPlayBytes = (setPlay << SET_PLAY_POS) & SET_PLAY_BITS;
    const uint16_t gameStateBytes = (gameState << GAME_STATE_POS) & GAME_STATE_BITS;
    const uint16_t gamePhaseBytes = (gamePhase << GAME_PHASE_POS) & GAME_PHASE_BITS;
    const uint16_t competitionTypeBytes =
        (competitionType << COMPETITION_TYPE_POS) & COMPETITION_TYPE_BITS;
    const uint16_t competitionPhaseBytes =
        (competitionPhase << COMPETITION_PHASE_POS) & COMPETITION_PHASE_BITS;
    const uint16_t firstHalfBytes =
        (static_cast<uint16_t>(firstHalf) << FIRST_HALF_POS) & FIRST_HALF_BITS;
    const uint16_t kickingTeamBytes =
        (static_cast<uint16_t>(kickingTeam) << KICKING_TEAM_POS) & KICKING_TEAM_BITS;

    writeVal<uint16_t>(data, setPlayBytes | gameStateBytes | gamePhaseBytes | competitionTypeBytes |
                                 competitionPhaseBytes | firstHalfBytes | kickingTeamBytes);
  }

  void GameStateStruct::read(const void*& data)
  {
    auto stateBytes = readVal<uint16_t>(data);
    setPlay = (stateBytes & SET_PLAY_BITS) >> SET_PLAY_POS;
    gameState = (stateBytes & GAME_STATE_BITS) >> GAME_STATE_POS;
    gamePhase = (stateBytes & GAME_PHASE_BITS) >> GAME_PHASE_POS;
    competitionType = (stateBytes & COMPETITION_TYPE_BITS) >> COMPETITION_TYPE_POS;
    competitionPhase = (stateBytes & COMPETITION_PHASE_BITS) >> COMPETITION_PHASE_POS;
    firstHalf = (stateBytes & FIRST_HALF_BITS) >> FIRST_HALF_POS;
    kickingTeam = (stateBytes & KICKING_TEAM_BITS) >> KICKING_TEAM_POS;
  }

  void NTPMessage::write(void*& data, uint32_t timestamp) const
  {
    // Note that NTPMessage::receiver is written somewhere else due to data compression

    // First 32 Bits are for the requestSent field, last 16 Bits are for the requestReceipt field
    assert(requestOrigination < 0xFFFFFFFFu);
    assert(timestamp - requestReceipt < 0xFFFFu);
    writeVal<uint32_t>(data, requestOrigination & std::numeric_limits<uint32_t>::max());
    writeVal<uint16_t>(data, (timestamp - requestReceipt) & std::numeric_limits<uint16_t>::max());
  }

  void NTPMessage::read(const void*& data, uint32_t timestamp)
  {
    // Note that NTPMessage::receiver is read somewhere else due to data compression

    requestOrigination = readVal<uint32_t>(data);
    requestReceipt = timestamp - static_cast<uint16_t>(readVal<uint16_t>(data));
  }


  StandardMessage::StandardMessage()
    : version(DS_STANDARD_MESSAGE_STRUCT_VERSION)
    , member(std::numeric_limits<decltype(member)>::max())
    , timestamp(0)
    , isPenalized(true)
    , isRobotPoseValid(false)
    , headYawAngle(0.f)
    , currentlyPerformingRole(Role::MAX)
    , timeWhenReachBall(std::numeric_limits<uint32_t>::max())
    , timeWhenReachBallStriker(std::numeric_limits<uint32_t>::max())
    , timeWhenBallLastSeen(std::numeric_limits<uint32_t>::max())
    , ballValidity(0)
    , timestampLastJumped(0)
    , lastTimeWhistleDetected(std::numeric_limits<uint32_t>::max())
    , robotMap()
    , requestsNTPMessage(false)
    , ntpMessages()
  {
    static_assert(DS_STANDARD_MESSAGE_STRUCT_VERSION == 5,
                  "Message version mismatch in StandardMessage constructor");

    // header initialization
    const char* headerInit = DS_STANDARD_MESSAGE_STRUCT_HEADER;
    assert(sizeof(*headerInit) * std::strlen(headerInit) == sizeof(header));

    for (unsigned int i = 0; i < sizeof(header); i++)
    {
      header[i] = headerInit[i];
    }

    // role assignment initialization
    roleAssignments[0] = Role::KEEPER;
    for (unsigned int i = 1; i < DS_STANDARD_MESSAGE_MAX_NUM_PLAYERS; i++)
    {
      roleAssignments[i] = Role::MAX;
    }
  }

  int StandardMessage::sizeOfDSMessage() const
  {
    static_assert(DS_STANDARD_MESSAGE_STRUCT_VERSION == 5,
                  "Message version mismatch in StandardMessage sizeOfDSMessage()");

    // clang-format off
    return sizeof(header) + sizeof(version) + sizeof(timestamp)
           + 2 // message length (not a member of this struct)
           + 1 // headYawAngle
           + 1 // timestampLastJumped
           + 2 // timeWhenReachBall
           + 2 // timeWhenReachBallStriker
           + 4 // timeWhenBallLastSeen
           + 4 // ball velocity
           + 1 // ball percepts per second
           + 2 // lastTimeWhistleDetected
           + GameStateStruct::sizeOf()
           + 4 // roleAssignments + currentlyPerformingRole
           + 1 // member, isPenalized, requestsNTPMessage, isRobotPoseValid
           + robotMap.sizeOf()
           + 1 // NTP receivers
           + static_cast<int>(ntpMessages.size()) * NTPMessage::sizeOf();
    // clang-format on
  }

  bool StandardMessage::read(const void* data)
  {
    static_assert(DS_STANDARD_MESSAGE_STRUCT_VERSION == 5,
                  "Message version mismatch in StandardMessage read()");

#ifndef NDEBUG
    const void* const begin = data; // For length check
#endif

    ntpMessages.clear();

    // check header
    for (unsigned int i = 0; i < sizeof(header); i++)
    {
      if (header[i] != readVal<const char>(data))
      {
        return false;
      }
    }

    // check version
    version = readVal<decltype(version)>(data);
    if (version != DS_STANDARD_MESSAGE_STRUCT_VERSION)
    {
      return false;
    }

    const void* const payloadBegin = data; // For length check

    const auto length = readVal<uint16_t>(data);

    timestamp = readVal<decltype(timestamp)>(data);

    headYawAngle = static_cast<float>(readVal<int8_t>(data)) / 180.f * pi;

    timestampLastJumped = timestamp - (static_cast<uint32_t>(readVal<const uint8_t>(data)) << 7u);
    timeWhenReachBall = timestamp + (static_cast<uint32_t>(readVal<const uint16_t>(data)) << 3u);
    timeWhenReachBallStriker =
        timestamp + (static_cast<uint32_t>(readVal<const uint16_t>(data)) << 3u);
    timeWhenBallLastSeen = readVal<const uint32_t>(data);
    ballVelocity[0] = static_cast<float>(readVal<const int16_t>(data));
    ballVelocity[1] = static_cast<float>(readVal<const int16_t>(data));
    ballValidity = static_cast<float>(readVal<uint8_t>(data)) / 255.f;
    const auto whistleTimeDiff = static_cast<uint32_t>(readVal<uint16_t>(data));
    // set lastTimeWhistleDetected to 0 if the diff exceeded the maximum value
    lastTimeWhistleDetected = whistleTimeDiff >= 0xFFFFu ? 0 : timestamp - whistleTimeDiff;

    gameState.read(data);

    const auto roleContainer = readVal<uint32_t>(data);
    currentlyPerformingRole =
        static_cast<Role>((roleContainer >> (4 * DS_STANDARD_MESSAGE_MAX_NUM_PLAYERS)) & 0xFu);
    for (unsigned int player = 0; player < DS_STANDARD_MESSAGE_MAX_NUM_PLAYERS; player++)
    {
      roleAssignments[player] = static_cast<Role>(
          (roleContainer >> (4 * (DS_STANDARD_MESSAGE_MAX_NUM_PLAYERS - player - 1))) & 0xFu);
    }

    auto boolContainer = readVal<uint8_t>(data);
    // according to write() there must not be more than 4 bools in here.
    assert((boolContainer & 0b11110000) == 0);
    requestsNTPMessage = (boolContainer & 1u) > 0;
    isRobotPoseValid = ((boolContainer >>= 1u) & 1u) > 0;
    isPenalized = ((boolContainer >>= 1u) & 1u) > 0;
    member = static_cast<decltype(member)>((boolContainer >>= 1u) & 1u);

    robotMap.read(data);

    const auto ntpReceiverContainer = readVal<uint8_t>(data);
    for (uint8_t i = 0; i < DS_STANDARD_MESSAGE_MAX_NUM_PLAYERS; i++)
    {
      if ((ntpReceiverContainer & (1u << i)) > 0)
      {
        NTPMessage msg;
        msg.receiver = i + 1;

        msg.read(data, timestamp);

        ntpMessages.emplace_back(msg);
      }
    }

    assert((reinterpret_cast<const char*>(data) - reinterpret_cast<const char*>(begin)) ==
           sizeOfDSMessage());

    return (reinterpret_cast<const char*>(data) - reinterpret_cast<const char*>(payloadBegin)) ==
           length;
  }

  void StandardMessage::write(void* data)
  {
    static_assert(DS_STANDARD_MESSAGE_STRUCT_VERSION == 5,
                  "Message version mismatch in StandardMessage write()");

#ifndef NDEBUG
    const void* const begin = data; // For length check
#endif

    for (unsigned int i = 0; i < sizeof(header); i++)
    {
      writeVal<char>(data, header[i]);
    }

    writeVal<decltype(version)>(data, version);
    // write payload length for length check on receiving side.
    writeVal<uint16_t>(data, sizeOfDSMessage() - sizeof(header) - sizeof(version));
    writeVal<decltype(timestamp)>(data, timestamp);

    // head yaw angle must be in degrees and must be in int8_t boundaries
    float headYawAngleFormatted = headYawAngle * 180 / pi;
    writeVal<int8_t>(data,
                     static_cast<int8_t>(std::max(std::min(headYawAngleFormatted, 127.f), -127.f)));

    // last jumped is only sent with 128ms precision
    // last jumped may not be greater than 250 (*128ms) otherwise it is clipped to 0xFF
    assert(timestampLastJumped <= timestamp);
    writeVal<uint8_t>(data, static_cast<uint8_t>(shiftAndClip<uint32_t>(
                                timestamp - timestampLastJumped, 250, 0xFF, 7)));

    // timeWhenReachBall is only sent with 8ms precision
    // timeWhenReachBall may not be greater than 0xFFFE (*8ms) otherwise it is clipped to 0xFFFF
    writeVal<uint16_t>(
        data, static_cast<uint16_t>(shiftAndClip<uint32_t>(
                  (std::max(timestamp, timeWhenReachBall) - timestamp), 0xFFFE, 0xFFFF, 3)));

    // timeWhenReachBallStriker is only sent with 8ms precision
    // timeWhenReachBallStriker may not be greater than 0xFFFC (*8ms) otherwise it is clipped to
    // 0xFFFD (the striker's time to reach ball should always be smaller than the normal time to
    // reach ball)
    writeVal<uint16_t>(
        data, static_cast<uint16_t>(shiftAndClip<uint32_t>(
                  (std::max(timestamp, timeWhenReachBallStriker) - timestamp), 0xFFFC, 0xFFFD, 3)));

    writeVal<decltype(timeWhenBallLastSeen)>(data, timeWhenBallLastSeen);

    writeVal<int16_t>(data, static_cast<int16_t>(ballVelocity[0]));
    writeVal<int16_t>(data, static_cast<int16_t>(ballVelocity[1]));
    writeVal<uint8_t>(data,
                      static_cast<uint8_t>(std::max(std::min(ballValidity, 0.f), 1.f) * 255.f));

    writeVal<uint16_t>(data, static_cast<uint16_t>(shiftAndClip<uint32_t>(
                                 timestamp - lastTimeWhistleDetected, 0xFFFE, 0xFFFF, 0)));

    gameState.write(data);

    // Writing the role assignments and currentlyPerformingRole into one 32 bit unsigned integer
    // Each assignment will be written as 4 bit leading to (4 * 6 + 4) = 28 Bits written into data.
    static_assert(static_cast<unsigned int>(Role::MAX) <= 16,
                  "Sending roles is not supported for more than 16 roles");
    static_assert(DS_STANDARD_MESSAGE_MAX_NUM_PLAYERS <= 7,
                  "Sending roles is not supported for more than 7 players");
    assert(static_cast<uint8_t>(currentlyPerformingRole) < static_cast<uint8_t>(Role::MAX));
    uint32_t roleContainer = static_cast<uint32_t>(currentlyPerformingRole);
    for (unsigned int player = 0; player < DS_STANDARD_MESSAGE_MAX_NUM_PLAYERS; player++)
    {
      // assert(static_cast<uint8_t>(roleAssignments[player]) < static_cast<uint8_t>(Role::MAX));
      (roleContainer <<= 4) |=
          (static_cast<decltype(roleContainer)>(roleAssignments[player]) & 0xFu);
    }
    writeVal<decltype(roleContainer)>(data, roleContainer);

    uint8_t boolContainer = 0;
    assert(member == HULKS_MEMBER || member == DEVIL_MEMBER);
    boolContainer |= static_cast<bool>(member) ? 1 : 0;
    (boolContainer <<= 1) |= isPenalized ? 1 : 0;
    (boolContainer <<= 1) |= isRobotPoseValid ? 1 : 0;
    (boolContainer <<= 1) |= requestsNTPMessage ? 1 : 0;
    writeVal<decltype(boolContainer)>(data, boolContainer);

    robotMap.write(data);

    // Each bit in this container represents a receiving robot (bit 0 represents player 1, bit 1
    // represents player 2, ...)
    static_assert(DS_STANDARD_MESSAGE_MAX_NUM_PLAYERS <= 8,
                  "NTP receiver bits are not adjusted for more than 8 players");
    // ntp messages need to be sorted so that the receivers can find their message.
    std::sort(ntpMessages.begin(), ntpMessages.end(),
              [&](const NTPMessage& a, const NTPMessage& b) { return a.receiver < b.receiver; });
    uint8_t ntpReceiverContainer = 0;
    for (const auto& ntpMessage : ntpMessages)
    {
      assert(ntpMessage.receiver - 1 < DS_STANDARD_MESSAGE_MAX_NUM_PLAYERS);
      ntpReceiverContainer |= 1 << (ntpMessage.receiver - 1);
    }
    writeVal<decltype(ntpReceiverContainer)>(data, ntpReceiverContainer);

    for (const auto& ntpMessage : ntpMessages)
    {
      ntpMessage.write(data, timestamp);
    }

    assert((reinterpret_cast<const char*>(data) - reinterpret_cast<const char*>(begin)) ==
           sizeOfDSMessage());
  }
} // namespace DevilSmash
