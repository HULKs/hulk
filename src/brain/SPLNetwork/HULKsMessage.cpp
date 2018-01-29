
#include "HULKsMessage.hpp"

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
    return sizeof(float) * 2                                // The current search position
           + sizeof(float) * 2 * positionSuggestions.size() // the suggested positions
           + 1;                                             // 'numberOfSuggestedPositions'
  }

  void BallSearchData::write(void* data) const
  {
#ifndef NDEBUG
    const void* const begin = data; // For size check only.
#endif

    writeVal<float>(data, currentSearchPosition.x());
    writeVal<float>(data, currentSearchPosition.y());

    writeVal<uint8_t>(data, positionSuggestions.size());

    for (unsigned int player = 0; player < positionSuggestions.size(); player++)
    {
      writeVal<float>(data, positionSuggestions[player].x());
      writeVal<float>(data, positionSuggestions[player].y());
    }

    // Check for size.
    assert((reinterpret_cast<char*>(data) - reinterpret_cast<const char* const>(begin)) == sizeOfBallSearchData());
  }

  void BallSearchData::read(const void* data)
  {
    currentSearchPosition.x() = readVal<const float>(data);
    currentSearchPosition.y() = readVal<const float>(data);

    unsigned int numberOfSuggestedPositions = readVal<const uint8_t>(data);
    positionSuggestions.resize(numberOfSuggestedPositions);

    for (unsigned int player = 0; player < numberOfSuggestedPositions; player++)
    {
      Vector2f position;
      position.x() = readVal<const float>(data);
      position.y() = readVal<const float>(data);
      positionSuggestions[player] = position;
    }
  }

  int HULKsMessage::sizeOfHULKsMessage() const
  {
    return ballSearchData.sizeOfBallSearchData() // all ball search data.
           + JOINTS::JOINTS_MAX                  // every joint has one uint8_t
           + 1;                                  // version tag uint8_t
  }

  void HULKsMessage::write(void* data) const
  {
    writeVal<uint8_t>(data, version);

    for (int joint = 0; joint < JOINTS::JOINTS_MAX; joint++)
    {
      writeVal<uint8_t>(data, jointStatus[joint]);
    }

    ballSearchData.write(data);
  }

  bool HULKsMessage::read(const void* data)
  {
    version = readVal<const uint8_t>(data);

    if (version != currentVersion)
    {
      return false;
    }

    for (int joint = 0; joint < JOINTS::JOINTS_MAX; joint++)
    {
      jointStatus[joint] = readVal<const uint8_t>(data);
    }
    ballSearchData.read(data);

    return true;
  }
}
