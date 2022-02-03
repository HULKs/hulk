#include "Brain/Network/SPLNetwork/HULKsMessage.hpp"
#include "Messages/RoboCupGameControlData.hpp"

namespace HULKs
{
  template <typename T>
  inline void writeVal(void*& data, T value)
  {
    *reinterpret_cast<T*>(data) = value;
    reinterpret_cast<char*&>(data) += sizeof(T);
  }

  template <typename T>
  inline T readVal(const void*& data)
  {
    const T val = *reinterpret_cast<const T*>(data);
    reinterpret_cast<const char*&>(data) += sizeof(T);
    return val;
  }

  int Obstacle::sizeOfObstacle()
  {
    static_assert(HULKsMessage::currentVersion == 5,
                  "The constructor is not adjusted for the current message version");
    return 5;
  }

  void Obstacle::write(void*& data, uint32_t timestamp) const
  {
    static_assert(HULKsMessage::currentVersion == 5,
                  "The constructor is not adjusted for the current message version");
#ifndef NDEBUG
    const void* const begin = data; // just for length check
#endif

    static_assert(static_cast<int>(ObstacleType::MAX) - 1 <= 8,
                  "Following does not work for that many ObstacleTypes. Adjust it.");

    writeVal<int16_t>(data, static_cast<int16_t>(((static_cast<int16_t>(center[0]) >> 2) & 0x3FFF) |
                                                 ((static_cast<int>(type) & 0xC) << 12)));
    writeVal<int16_t>(data, static_cast<int16_t>(((static_cast<int16_t>(center[1]) >> 2) & 0x3FFF) |
                                                 ((static_cast<int>(type) & 0x3) << 14)));

    const uint32_t timestampLastSeenDiff64 = (timestamp - timestampLastSeen) >> 6;
    writeVal<uint8_t>(data, static_cast<uint8_t>(
                                timestampLastSeenDiff64 > 0xFE ? 0xFF : timestampLastSeenDiff64));

    assert((reinterpret_cast<const char*>(data) - reinterpret_cast<const char*>(begin)) ==
           sizeOfObstacle());
  }

  void Obstacle::read(const void*& data, uint32_t timestamp)
  {
    static_assert(HULKsMessage::currentVersion == 5,
                  "The constructor is not adjusted for the current message version");
    const int16_t center0Struct = readVal<const int16_t>(data);
    const int16_t center1Struct = readVal<const int16_t>(data);

    center[0] = static_cast<float>(static_cast<int16_t>(center0Struct << 2));
    center[1] = static_cast<float>(static_cast<int16_t>(center1Struct << 2));

    type = static_cast<ObstacleType>((static_cast<uint16_t>(center0Struct & 0xC000) >> 12) |
                                     (static_cast<uint16_t>(center1Struct & 0xC000) >> 14));

    const uint8_t timestampLastSeenDiff64 = readVal<const uint8_t>(data);
    timestampLastSeen = timestamp - (static_cast<uint32_t>(timestampLastSeenDiff64) << 6);
  }

  BallSearchData::BallSearchData()
  {
    currentSearchPosition = Vector2f(0, 0);
  }

  int BallSearchData::sizeOfBallSearchData() const
  {
    // the amount of suggestions sent.
    uint8_t sizeOfSuggestions = sizeof(float) * 2 * MAX_NUM_PLAYERS;

    return sizeof(float) * 2   // currentSearchPosition (Vector2f)
           + sizeof(uint8_t)   // positionSuggestionsValidity
           + sizeOfSuggestions // the suggested positions
           + sizeof(uint32_t)  // timestampBallSearchMapUnreliable
           + sizeof(uint8_t)   // mostWisePlayerNumber
           + 1;                // availableForSearch
  }

  void BallSearchData::write(void*& data) const
  {
#ifndef NDEBUG
    const void* const begin = data; // For size check only.
#endif

    writeVal<float>(data, currentSearchPosition.x());
    writeVal<float>(data, currentSearchPosition.y());

    writeVal<uint8_t>(data, positionSuggestionsValidity);

    for (const auto& positionSuggestion : positionSuggestions)
    {
      writeVal<float>(data, positionSuggestion.x());
      writeVal<float>(data, positionSuggestion.y());
    }

    // Fill the message with nonsense data since it should have a defined size
    // at the receiver side. This data will be marked as *invalid*.
    for (auto i = static_cast<uint8_t>(positionSuggestions.size()); i < MAX_NUM_PLAYERS; i++)
    {
      writeVal<float>(data, 0.f);
      writeVal<float>(data, 0.f);
    }

    writeVal<unsigned int>(data, timestampBallSearchMapUnreliable);
    writeVal<bool>(data, availableForSearch);
    writeVal<uint8_t>(data, mostWisePlayerNumber);

    // Check for size.
    assert((reinterpret_cast<const char*>(data) - reinterpret_cast<const char*>(begin)) ==
           sizeOfBallSearchData());
  }

  void BallSearchData::read(const void*& data)
  {
    currentSearchPosition.x() = readVal<const float>(data);
    currentSearchPosition.y() = readVal<const float>(data);

    // positionSuggestionsValidity stores a valid flag for every player's positionSuggestion. If
    // there are more than 8 players, the valid flags will not fit into the receiver type (uint8_t).
    static_assert(MAX_NUM_PLAYERS <= 8,
                  "positionSuggestionsValidity currently only supports 8 players.");
    positionSuggestionsValidity = readVal<uint8_t>(data);

    positionSuggestions.resize(MAX_NUM_PLAYERS);

    // Read all position suggestions (even if they are garbage / invalid)
    for (unsigned int player = 0; player < MAX_NUM_PLAYERS; player++)
    {
      Vector2f position;
      position.x() = readVal<const float>(data);
      position.y() = readVal<const float>(data);
      positionSuggestions[player] = position;
    }

    timestampBallSearchMapUnreliable = readVal<uint32_t>(data);
    availableForSearch = readVal<const bool>(data);
    mostWisePlayerNumber = readVal<uint8_t>(data);
  }

  HULKsMessage::HULKsMessage()
    : version(currentVersion)
    , timestamp(0)
    , walkingTo()
    , passTarget(0)
    , obstacles()
    , ballSearchData()
  {
    // header initialization
    const char* headerInit = HULKS_STANDARD_MESSAGE_STRUCT_HEADER;
    assert(sizeof(*headerInit) * std::strlen(headerInit) == sizeof(header));

    for (unsigned int i = 0; i < sizeof(header); i++)
    {
      header[i] = headerInit[i];
    }
  }

  int HULKsMessage::sizeOfHULKsMessage() const
  {
    // clang-format off
    return sizeof(header)                          // clang format is a bitch
           + 3 * sizeof(float)                     // walkingTo
           + std::min(int(obstacles.size()), HULKS_STANDARD_MESSAGE_MAX_NUM_OF_OBSTACLES) *
                 Obstacle::sizeOfObstacle()        // the obstacle data
           + ballSearchData.sizeOfBallSearchData() // all ball search data.
           + 1                                     // version tag uint8_t
           + 1;                                    // passTarget & numOfObstacles
    // clang-format on
  }

  void HULKsMessage::write(void* data) const
  {
#ifndef NDEBUG
    const void* const begin = data; // For size check only.
#endif

    for (unsigned int i = 0; i < sizeof(header); i++)
    {
      writeVal<char>(data, header[i]);
    }

    writeVal<unsigned char>(data, version);
    const uint8_t numObstacles =
        std::min(static_cast<uint8_t>(HULKS_STANDARD_MESSAGE_MAX_NUM_OF_OBSTACLES),
                 static_cast<uint8_t>(obstacles.size()));

    static_assert(HULKS_STANDARD_MESSAGE_MAX_NUM_PLAYERS < 16,
                  "Pass target is not implemented for that many players");
    static_assert(HULKS_STANDARD_MESSAGE_MAX_NUM_OF_OBSTACLES <= 16,
                  "Too many obstacles for this implementation");
    assert(passTarget <= HULKS_STANDARD_MESSAGE_MAX_NUM_PLAYERS);
    assert(numObstacles <= HULKS_STANDARD_MESSAGE_MAX_NUM_OF_OBSTACLES);
    writeVal<uint8_t>(data, passTarget | (numObstacles << 4));
    writeVal<float>(data, walkingTo.x());
    writeVal<float>(data, walkingTo.y());
    writeVal<float>(data, walkingTo.angle());

    for (unsigned int i = 0; i < numObstacles; i++)
    {
      obstacles[i].write(data, timestamp);
    }

    ballSearchData.write(data);

    // Check for size.
    assert((reinterpret_cast<const char*>(data) - reinterpret_cast<const char*>(begin)) ==
           sizeOfHULKsMessage());
  }

  bool HULKsMessage::read(const void* data)
  {
    obstacles.clear();

    // check header
    for (unsigned int i = 0; i < sizeof(header); i++)
    {
      if (header[i] != readVal<const char>(data))
      {
        return false;
      }
    }

    version = readVal<const uint8_t>(data);

    if (version != currentVersion)
    {
      return false;
    }

    const auto container = readVal<uint8_t>(data);
    passTarget = container & 0xFu;
    const uint8_t numObstacles = (container >> 4) & 0xFu;
    walkingTo.x() = readVal<const float>(data);
    walkingTo.y() = readVal<const float>(data);
    walkingTo.angle() = readVal<const float>(data);

    for (unsigned int i = 0; i < numObstacles; i++)
    {
      Obstacle obstacle{};
      obstacle.read(data, timestamp);
      obstacles.push_back(obstacle);
    }

    ballSearchData.read(data);

    return true;
  }
} // namespace HULKs
