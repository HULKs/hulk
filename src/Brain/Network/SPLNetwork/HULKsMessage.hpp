/**
 * @file HULKsMessage.hpp
 *
 * This file contains information shared between HULKs robots that are not part of the
 * DevilSmashStandardMessage.
 *
 *
 * UNITS
 *
 * - Distances:     Millimeters (mm)
 * - Angles:        Radians
 * - Time:          Milliseconds (ms)
 * - Speed:         Speed (m/s)
 * - Timestamps:    Milliseconds since system/software start (ms)
 *                  Offset of timestamps are determined via NTP messages
 *
 *
 * VALUE RANGES
 *
 * The values stored in the DS message are not necessarily streamed as a whole.
 * If a value is not streamed in its natural range, a comment will indicate how the value is
 * interpreted.
 * The general comment pattern is [rangeFrom..rangeTo (precision)].
 * E.g.:
 *   /// no comment    (This will be streamed as full 4 Byte)
 *   uint32_t value0;
 *
 *   /// [2..12]    (This will be streamed with a minimum value of 2 and a maximum value of 12)
 *   uint32_t value1;
 *
 *   /// [2..12 (2)]    (This will be streamed with with a precision of 2, a minimum value of 2 and
 *       a maximum value of 12)
 *   uint32_t value2;
 *
 *   /// [delta 0..-10]    (This will be streamed in relation to the timestamp of the message in the
 *       range of 0 to -10)
 *   uint32_t time1
 *
 *   /// [delta 0..-10 (64ms)] timestamp (This will be streamed in relation to the
 *       timestamp of the message in the range of 0 to -10, unit of the values is 64ms
 *   uint32_t time1
 */

#pragma once

#include <cstdint>

#include "Tools/Math/Eigen.hpp"
#include "Tools/Math/Pose.hpp"

#define HULKS_STANDARD_MESSAGE_STRUCT_HEADER "HULK"
#define HULKS_STANDARD_MESSAGE_MAX_NUM_OF_OBSTACLES 16
#define HULKS_STANDARD_MESSAGE_MAX_NUM_PLAYERS 6

namespace HULKs
{
  /**
   * The type of an obstacle.
   */
  enum class ObstacleType : uint8_t
  {
    UNKNOWN,
    SOME_ROBOT,
    OPPONENT,
    TEAM_ROBOT,
    FALLEN_SOME_ROBOT,
    FALLEN_OPPONENT,
    FALLEN_TEAMMATE,
    MAX
  };

  /**
   * The definition of an Obstacle as it is shared between HULKs robots.
   */
  struct Obstacle : public Uni::To, public Uni::From
  {
    /// [short (4mm)] the obstacle center in robot coordinates
    /// x goes to front
    /// y goes to left
    std::array<float, 2> center;

    /// [delta 0..-16384 (64ms)] timestamp
    /// the time when the obstacle was last seen
    uint32_t timestampLastSeen;
    /// [0..(Obstacle::MAX-2)] the obstacle type
    ObstacleType type;


    /**
     * @brief sizeOfObstacle returns the size of this struct when written into the actual message
     * @return the size of this struct when written into the actual message
     */
    static int sizeOfObstacle();

    /**
     * @brief write writes the information of this struct into the given data field
     * @param data the data field to write this struct's information into
     * @param timestamp the timestamp of the message this obstacle will be sent with
     */
    void write(void*& data, uint32_t timestamp) const;

    /**
     * @brief read stores the information from the given data field into this struct
     * @param data the data field to read the information from
     * @param timestamp the timestamp of the message this obstacle was received with
     */
    void read(const void*& data, uint32_t timestamp);

    void toValue(Uni::Value& value) const override
    {
      value = Uni::Value(Uni::ValueType::OBJECT);
      value["center"] << center;
      value["timestampLastSeen"] << timestampLastSeen;
      value["type"] << static_cast<int>(type);
    }

    void fromValue(const Uni::Value& value) override
    {
      value["center"] >> center;
      value["timestampLastSeen"] >> timestampLastSeen;
      int valueRead;
      value["type"] >> valueRead;
      type = static_cast<ObstacleType>(valueRead);
    }
  };

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
    /// the player with the oldest, continuously updated map.
    uint8_t mostWisePlayerNumber;
    /// if the robot is available for searching for the ball.
    bool availableForSearch;

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
    /// HULKS_STANDARD_MESSAGE_STRUCT_HEADER
    char header[4];
    /// the version of the HULKsMessage that this robot sends should be
    /// increased when e.g. a member is added to this struct
    static constexpr std::uint8_t currentVersion = 5;
    /// the version of this message
    std::uint8_t version;
    /// the timestamp when this message was created.
    /// WARNING: This must be set before calling read - it is sent via the DevilSmashStandardMessage
    uint32_t timestamp;
    /// the pose the robot is currently walking to.
    Pose walkingTo;
    /// [0..HULKS_STANDARD_MESSAGE_MAX_NUM_PLAYERS]
    /// the pass target of this player (0 for none).
    uint8_t passTarget;
    /// The obstacles seen by a robot
    std::vector<Obstacle> obstacles;
    /// The ball search data needed and produced by the BallSearchPositionProvider
    BallSearchData ballSearchData;

    /**
     * @brief HULKsMessage initializes members
     */
    HULKsMessage();

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
