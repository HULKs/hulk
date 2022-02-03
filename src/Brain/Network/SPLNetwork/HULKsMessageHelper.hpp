#pragma once

#include "Brain/Network/SPLNetwork/HULKsMessage.hpp"
#include "Data/ObstacleData.hpp"

namespace HULKs
{
  /**
   * @brief hmObstacleTypeToObstacleType converts a HULKsMessage obstacle type to obstacle type
   * @param obstacleType the HULKsMessage obstacle type
   * @return the obstacle type
   */
  inline ::ObstacleType hmObstacleTypeToObstacleType(const ObstacleType obstacleType)
  {
    switch (obstacleType)
    {
      case ObstacleType::UNKNOWN:
        return ::ObstacleType::UNKNOWN;
      case ObstacleType::SOME_ROBOT:
        return ::ObstacleType::ANONYMOUS_ROBOT;
      case ObstacleType::OPPONENT:
        return ::ObstacleType::HOSTILE_ROBOT;
      case ObstacleType::TEAM_ROBOT:
        return ::ObstacleType::TEAM_ROBOT;
      case ObstacleType::FALLEN_SOME_ROBOT:
        return ::ObstacleType::FALLEN_ANONYMOUS_ROBOT;
      case ObstacleType::FALLEN_OPPONENT:
        return ::ObstacleType::FALLEN_HOSTILE_ROBOT;
      case ObstacleType::FALLEN_TEAMMATE:
        return ::ObstacleType::FALLEN_TEAM_ROBOT;
      default:
        return ::ObstacleType::UNKNOWN;
    }
  }

  /**
   * @brief obstacleTypeToHMObstacleType converts obstacle type to HULKsMessage obstacle type
   * @param obstacleType the obstacle type
   * @return the HULKsMessage obstacle type
   */
  inline ObstacleType obstacleTypeToHMObstacleType(const ::ObstacleType obstacleType)
  {
    switch (obstacleType)
    {
      case ::ObstacleType::UNKNOWN:
        return ObstacleType::UNKNOWN;
      case ::ObstacleType::ANONYMOUS_ROBOT:
        return ObstacleType::SOME_ROBOT;
      case ::ObstacleType::HOSTILE_ROBOT:
        return ObstacleType::OPPONENT;
      case ::ObstacleType::TEAM_ROBOT:
        return ObstacleType::TEAM_ROBOT;
      case ::ObstacleType::FALLEN_ANONYMOUS_ROBOT:
        return ObstacleType::FALLEN_SOME_ROBOT;
      case ::ObstacleType::FALLEN_HOSTILE_ROBOT:
        return ObstacleType::FALLEN_OPPONENT;
      case ::ObstacleType::FALLEN_TEAM_ROBOT:
        return ObstacleType::FALLEN_TEAMMATE;
      case ::ObstacleType::BALL:
        return ObstacleType::MAX;
      case ::ObstacleType::FREE_KICK_AREA:
        return ObstacleType::MAX;
      case ::ObstacleType::INVALID:
        return ObstacleType::MAX;
      case ::ObstacleType::OBSTACLETYPE_MAX:
        return ObstacleType::MAX;
      default:
        return ObstacleType::MAX;
    }
  }
} // namespace HULKs
