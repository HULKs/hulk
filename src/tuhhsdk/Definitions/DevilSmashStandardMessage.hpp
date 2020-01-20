/**
 * @file DevilSmashStandardMessage.hpp
 *
 * This file contains everything needed to communicate between Nao Devils and HULKs robots (mixed
 * team competition 2019) when sent in the Data field of a SPLStandardMessage.
 *
 *
 * UNITS
 *
 * - Distances:     Millimeters (mm)
 * - Angles:        Radians
 * - Time:          Milliseconds (ms)
 * - Speed:         Speed (mm/s)
 * - Timestamps:    Milliseconds since system/software start (ms)
 *                  Offset of timestamps are determined via NTP messages
 *
 *
 * COORDINATE SYSTEMS
 *
 * - Absolute coordinates (aka field coordinates):
 *   - origin is in the middle of the center circle.
 *   - x axis points at the goal of the enemy
 *   - y axis is aligned accordingly:
 *
 *   ___________________________
 *  |             |             |
 *  |___        y ^          ___|
 *  |   |         |         | E |
 * .| O |         |         | N |.
 * || W |         o---->    | E ||
 * '| N |         |    x    | M |'
 *  |___|         |         |_Y_|
 *  |             |             |
 *  |_____________|_____________|
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
 *   /// [delta 0..10]    (This will be streamed in relation to the timestamp of the message in the
 *       range of 0 to 10)
 *   uint32_t time1
 *
 *   /// [delta 0..10 (64ms)] time since msg timestamp (This will be streamed in relation
 *       to the timestamp of the message in the range of 0 to 10, unit of the values is 64ms
 *   uint32_t time1
 */


#pragma once

#include <algorithm>
#include <cassert>
#include <limits>
#include <stdint.h>
#include <vector>

#define DS_STANDARD_MESSAGE_STRUCT_HEADER "DESM"
#define DS_STANDARD_MESSAGE_STRUCT_VERSION 5
#define DS_STANDARD_MESSAGE_MAX_NUM_PLAYERS 6
#define DS_STANDARD_MESSAGE_MAX_ROBOTS_IN_MAP 12

#define DEVIL_MEMBER 0
#define HULKS_MEMBER 1

namespace DevilSmash
{
  /**
   * @brief The roles that can be assigned to a robot.
   */
  enum class Role : uint8_t
  {
    NONE,               /// Indicates that there is no role assignment for this player
    KEEPER,             /// Keeper, stands in the own penalty area
    REPLACEMENT_KEEPER, /// The player that replaces the keeper (in case the keeper is penalized)
    DEFENDER_LEFT,      /// Defensive player, must not touch the penalty area in most situations
    DEFENDER_RIGHT,     /// Defensive player, must not touch the penalty area in most situations
    PUNISHER,           /// Stands aggressively in the enemy half and waits for passes
    SUPPORT,            /// Supports the striker (behind the striker)
    STRIKER,            /// The guy that plays the ball
    MAX                 /// Value that indicates that something has gone wrong
    // TODO: Add / modify roles according to github discussions
  };

  /**
   * @brief the map of all robots seen by a player
   */
  struct RobotMap
  {
    /**
     * @brief Robot A description of a robot on the field
     */
    struct Robot
    {
      enum class Type : uint8_t
      {
        UNKNOWN,  /// Robot type is not known
        TEAMMATE, /// Robot is a teammate
        ENEMY,    /// Robot is an enemy
        MAX       /// Something has gone wrong!
      };
      /// will be serialized as int16_t in 1/4mm
      /// The position in absolute field coordinates
      float x, y;
      /// the robot type
      Type type;
    };

    /// map contains all robots that were detected by a player.
    std::vector<Robot> map;

    /**
     * @brief returns the size of this struct when written into the SPLMessage's data field.
     * @return size in bytes
     */
    int sizeOf() const;

    /**
     * @brief stores all data from this struct into the given data field
     * @param data pointer to the data field to store this struct in.
     */
    void write(void*& data) const;

    /**
     * @brief stores all content from data into this struct.
     * @param data The data to be stored.
     */
    void read(const void*& data);
  };

  /**
   * @brief the current game state the robot is aware of
   *
   * Needed as we only send the set positions during READY, SET.
   * Also serves as a backup in case of package loss from the game controller
   */
  struct GameStateStruct
  {
    // The positions of the single pieces of information inside the 2 byte data field.
    const uint8_t SET_PLAY_POS = 0u;
    const uint8_t GAME_STATE_POS = 3u;
    const uint8_t GAME_PHASE_POS = 6u;
    const uint8_t COMPETITION_TYPE_POS = 8u;
    const uint8_t COMPETITION_PHASE_POS = 10u;
    const uint8_t FIRST_HALF_POS = 11u;
    const uint8_t KICKING_TEAM_POS = 12u;
    // the bit masks to use for placing the bits into the 2 byte data field
    // clang-format off
    const uint16_t SET_PLAY_BITS =          0b0000000000000111;
    const uint16_t GAME_STATE_BITS =        0b0000000000111000;
    const uint16_t GAME_PHASE_BITS =        0b0000000011000000;
    const uint16_t COMPETITION_TYPE_BITS =  0b0000001100000000;
    const uint16_t COMPETITION_PHASE_BITS = 0b0000010000000000;
    const uint16_t FIRST_HALF_BITS =        0b0000100000000000;
    const uint16_t KICKING_TEAM_BITS =      0b0001000000000000;
    // clang-format on

    /// [0..7] set play
    uint16_t setPlay;
    /// [0..7] game state
    uint16_t gameState;
    /// [0..1] game phase
    uint16_t gamePhase;
    /// [0..3] competition type
    uint16_t competitionType;
    /// [0..1] competition phase
    uint16_t competitionPhase;
    /// [0..1] first half
    bool firstHalf;
    /// [0..1] whether we are the kicking team
    /// Note that this might differ from the game controller messages in case of detected referee
    /// mistakes
    bool kickingTeam;

    /**
     * @brief the size of the data to be sent / received
     * @return yes
     */
    static int sizeOf()
    {
      return 2;
    }

    /**
     * @brief stores all data from this struct into the given data field
     * @param data pointer to the data field to store this struct in.
     */
    void write(void*& data) const;

    /**
     * @brief stores all content from data into this struct.
     * @param data The data to be stored.
     */
    void read(const void*& data);
  };

  /**
   * @brief NTPMessage a simple NTP response
   *
   * This message should be sent when a robot asked for a NTP message via a ntp request.
   */
  struct NTPMessage
  {
    /// [0..0xFFFFFFFF] timestamp (max 1193.04 hours)
    /// timestamp of the generation of the request
    uint32_t requestOrigination;
    /// [delta 0..0xFFFF] time since msg timestamp (max 1 minute)
    /// timestamp of the receipt of the request
    uint32_t requestReceipt;
    /// [1..DS_STANDARD_MESSAGE_MAX_NUM_PLAYERS] the robot to which the message is sent
    uint8_t receiver;

    /**
     * @brief sizeOf returns the size this struct needs in data when performing write()
     * @return the size in bytes
     */
    static int sizeOf()
    {
      return 6; // does not include receiver for space efficiency reasons.
    }

    /**
     * @brief stores all data from this struct into the given data field
     *
     * WARNING: write does not write the receiver byte into data. This should to be done separately
     * as there are more efficient ways to store the receiver.
     *
     * @param data pointer to the data field to store this struct in.
     * @param timestamp The timestamp of this message for reference
     */
    void write(void*& data, uint32_t timestamp) const;

    /**
     * @brief stores all content from data into this struct.
     *
     * WARNING: read does not read the receiver byte from data (like write does not write it).
     *
     * @param data The data to be stored.
     * @param timestamp The timestamp of this message for reference
     */
    void read(const void*& data, uint32_t timestamp);
  };

  /**
   * @brief DevilSmashStandardMessage for well defined inter team communication
   *
   * This message should be placed at the beginning of the data field of every SPL standard message
   */
  struct StandardMessage
  {
    /// DS_STANDARD_MESSAGE_STRUCT_HEADER
    char header[4];
    /// DS_STANDARD_MESSAGE_STRUCT_VERSION
    uint8_t version;
    /// [0..1] either DEVIL_MEMBER or HULK_MEMBER
    uint8_t member;
    /// timestamp, when this message was sent
    uint32_t timestamp;

    /// Whether this robot is penalized or not.
    /// Note that this information might differ from the game controller information! The HULK
    /// robots may penalize themselves for short periods of time in case of (assistant) referee
    /// mistakes.
    bool isPenalized;

    /// Whether the pose this robots sends is valid or not.
    /// As an example, this can be false in case the robot is not sure about self localization
    bool isRobotPoseValid;

    /// [-127..127 (1 deg)]
    /// the current measurement of the head join: HeadYaw
    float headYawAngle;

    /// the role that this robot is currently performing
    Role currentlyPerformingRole;
    /// the role per robot (player number -1 used as index) calculated by this robot
    Role roleAssignments[DS_STANDARD_MESSAGE_MAX_NUM_PLAYERS];

    /// The current game state this robot is aware of
    GameStateStruct gameState;

    /// [delta 0..0xFFFE (8ms)] relative to msg timestamp
    /// the time this robot needs to play the ball (includes getting around the ball)
    /// Must be greater than the message timestamp!
    uint32_t timeWhenReachBall;
    /// [delta 0..0xFFFC (8ms)] relative to msg timestamp
    /// the time this robot needs to play the ball (includes getting around the ball) but
    /// with the striker bonus (usually this timestamp is closer to "now" than the normal time to
    /// reach ball).
    /// Must be greater than the message timestamp!
    uint32_t timeWhenReachBallStriker;

    /// timestamp when the ball was last seen
    uint32_t timeWhenBallLastSeen;

    /// [int16_t value range]
    /// velocity of the ball in millimeters per second.
    float ballVelocity[2];
    /// [0..255]
    /// the validity of the ball. Should be between 0.f and 1.f
    float ballValidity;

    /// [delta 0..255 (128ms)] time since msg timestamp
    /// Describes the last time the robot's self localization corrected with a bigger update than
    /// normal (i.e. position estimate changed more than 1m since last cycle)
    uint32_t timestampLastJumped;

    /// [delta 0..0xFFFE] time since msg timestamp, last time the whistle was detected
    uint32_t lastTimeWhistleDetected;

    /// the robot map of this player
    RobotMap robotMap;

    /// whether we request a ntp message from our team mates.
    bool requestsNTPMessage;
    /// all ntp messages this robot sends to his teammates in response to their requests
    std::vector<NTPMessage> ntpMessages;

    /**
     * @brief StandardMessage initializes members
     */
    StandardMessage();

    /**
     * @brief returns the size of this message
     *
     * Only VALID iff the struct has been filled with meaningful data as the size varies depending
     * on the game state etc.
     *
     * @return the size of this message when it is written via write(data) given in bytes
     */
    int sizeOfDSMessage() const;

    /**
     * @brief Puts all data of this struct into a compressed data package
     *
     * Note: This is not const as it sorts the ntp messages
     *
     * @param data pointer to the data field to write this struct into. Must be at least as big as
     * sizeOfDSMessage
     */
    void write(void* data);

    /**
     * @brief Extracts all information from a compressed data package into this struct's members.
     * @param data pointer to the data field to read the data from.
     * @return true on success
     */
    bool read(const void* data);
  };
} // namespace DevilSmash
