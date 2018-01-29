#pragma once

#include <cstdint>

#include "Modules/NaoProvider.h"
#include "Tools/Math/Eigen.hpp"


namespace HULKs
{
  struct BallSearchData
  {
    /**
     * @brief Constructor
     */
    BallSearchData();

    /// The current search position of the sending robot. == Current position if not searching!
    Vector2f currentSearchPosition;
    /// The suggested positions to search for a ball. One position per robot: Index = team player number.
    VecVector2f positionSuggestions;

    /**
     * The size (bytes) of the ballSearchData struct. Needed for sizechecks while write() read().
     * @return int
     */
    int sizeOfBallSearchData() const;

    /**
     * For converting this struct for communication usage.
     * @param data Pointer to store the struct data to.
     */
    void write(void* data) const;

    /**
     * For converting a message into this struct.
     * @param data Pointer to the data to save to this struct.
     */
    void read(const void* data);
  };

  struct HULKsMessage
  {
    /// the version of the HULKsMessage that this robot sends should be increased when e.g. a member is added to this struct
    static constexpr std::uint8_t currentVersion = 3;
    /// the version is set to the current version by default
    std::uint8_t version = currentVersion;
    /// the statuses of the joints
    std::uint8_t jointStatus[JOINTS::JOINTS_MAX];
    /// The ball search data needed and produced by the BallSearchPositionProvider
    BallSearchData ballSearchData;

    /**
     * The size (bytes) of the hulks message.
     * @return int number of bytes
     */
    int sizeOfHULKsMessage() const;

    /**
     * For converting this struct for communication usage.
     * @param data Pointer to store the struct data to.
     */
    void write(void* data) const;

    /**
     * For converting a message into this struct.
     * @param data Pointer to the data to save to this struct.
     * @return If the version matched (and read was successfully executed).
     */
    bool read(const void* data);
  };
}
