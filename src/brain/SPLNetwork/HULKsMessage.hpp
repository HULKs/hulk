#pragma once

#include <cstdint>

#include "Modules/NaoProvider.h"
#include "Tools/Math/Eigen.hpp"
#include "Tools/Math/Pose.hpp"


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
    /// Each bit represents the validity of one 'positionsSuggestion' (bitwise mapping to index of
    /// the positionSuggestion)
    uint8_t positionSuggestionsValidity;
    /// The suggested positions to search for a ball. One position per robot: Index = team player
    /// number.
    VecVector2f positionSuggestions;
    /// the timestamp of the last time the map was not reliable due to penalties etc.
    uint32_t timestampBallSearchMapUnreliable;
    /// if the robot is available for searching for the ball.
    bool availableForSearch;
    /// the player with the oldest, continously updated map.
    uint8_t mostWisePlayerNumber;

    /**
     * The size (bytes) of the ballSearchData struct. Needed for sizechecks while write() read().
     * @return int
     */
    int sizeOfBallSearchData() const;

    /**
     * For converting this struct for communication usage.
     * @param data Pointer to store the struct data to.
     */
    void write(void*& data) const;

    /**
     * For converting a message into this struct.
     * @param data Pointer to the data to save to this struct.
     */
    void read(const void*& data);
  };

  struct HULKsMessage
  {
    /// the version of the HULKsMessage that this robot sends should be
    /// increased when e.g. a member is added to this struct
    static constexpr std::uint8_t currentVersion = 4;
    /// the version is set to the current version by default
    std::uint8_t version = currentVersion;
    /// If the robot is confident about it's self localization.
    bool isPoseValid;
    /// the pose the robot is currently walking to.
    Pose walkingTo;
    /// velocity of the ball in meters per second.
    float ballVel[2];
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
} // namespace HULKs
