#include "HULKsMessage.hpp"

#include "Definitions/RoboCupGameControlData.h"

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

  BallSearchData::BallSearchData()
  {
    currentSearchPosition = Vector2f(0, 0);
  }

  int BallSearchData::sizeOfBallSearchData() const
  {
    // the amount of suggestions sent.
    uint8_t sizeOfSuggestions = sizeof(float) * 2 * MAX_NUM_PLAYERS;

    return sizeof(float) * 2         // currentSearchPosition (Vector2f)
           + sizeof(uint8_t)         // positionSuggestionsValidity
           + sizeOfSuggestions       // the suggested positions
           + sizeof(uint32_t)        // timestampBallSearchMapUnreliable
           + sizeof(uint8_t)         // mostWisePlayerNumber
           + 1;                      // availableForSearch
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
    assert((reinterpret_cast<const char*>(data) - reinterpret_cast<const char*>(begin)) == sizeOfBallSearchData());
  }

  void BallSearchData::read(const void*& data)
  {
    currentSearchPosition.x() = readVal<const float>(data);
    currentSearchPosition.y() = readVal<const float>(data);

    // positionSuggestionsValidity stores a valid flag for every player's positionSuggestion. If there are
    // more than 8 players, the valid flags will not fit into the receiver type (uint8_t).
    static_assert(MAX_NUM_PLAYERS <= 8, "positionSuggestionsValidity currently only supports 8 players.");
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

  int HULKsMessage::sizeOfHULKsMessage() const
  {
    return 3 * sizeof(float)                        // walkingTo
           + 2 * sizeof(float)                      // ballVel
           + ballSearchData.sizeOfBallSearchData()  // all ball search data.
           + JOINTS::JOINTS_MAX                     // every joint has one uint8_t
           + 2;                                     // version tag uint8_t AND isPoseValid
  }

  void HULKsMessage::write(void* data) const
  {
#ifndef NDEBUG
    const void* const begin = data; // For size check only.
#endif

    writeVal<unsigned char>(data, version);
    writeVal<bool>(data, isPoseValid);
    writeVal<float>(data, walkingTo.position.x());
    writeVal<float>(data, walkingTo.position.y());
    writeVal<float>(data, walkingTo.orientation);
    writeVal<float>(data, ballVel[0]);
    writeVal<float>(data, ballVel[1]);

    ballSearchData.write(data);

    for (unsigned char jS : jointStatus)
    {
      writeVal<uint8_t>(data, jS);
    }

    // Check for size.
    assert((reinterpret_cast<const char*>(data) - reinterpret_cast<const char*>(begin)) == sizeOfHULKsMessage());
  }

  bool HULKsMessage::read(const void* data)
  {
    version = readVal<const uint8_t>(data);

    if (version != currentVersion)
    {
      return false;
    }

    isPoseValid = readVal<const bool>(data);
    walkingTo.position.x() = readVal<const float>(data);
    walkingTo.position.y() = readVal<const float>(data);
    walkingTo.orientation = readVal<const float>(data);
    ballVel[0] = readVal<const float>(data);
    ballVel[1] = readVal<const float>(data);

    ballSearchData.read(data);

    for (unsigned char& jS : jointStatus)
    {
      jS = readVal<const uint8_t>(data);
    }

    return true;
  }
}
