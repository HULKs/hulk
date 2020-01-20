#pragma once

#include "Data/ObstacleData.hpp"
#include "Network/SPLNetwork/HULKsMessage.hpp"

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
      case ObstacleType::goalpost:
        return ::ObstacleType::GOAL_POST;
      case ObstacleType::unknown:
        return ::ObstacleType::UNKNOWN;
      case ObstacleType::someRobot:
        return ::ObstacleType::ANONYMOUS_ROBOT;
      case ObstacleType::opponent:
        return ::ObstacleType::HOSTILE_ROBOT;
      case ObstacleType::teammate:
        return ::ObstacleType::TEAM_ROBOT;
      case ObstacleType::fallenSomeRobot:
        return ::ObstacleType::FALLEN_ANONYMOUS_ROBOT;
      case ObstacleType::fallenOpponent:
        return ::ObstacleType::FALLEN_HOSTILE_ROBOT;
      case ObstacleType::fallenTeammate:
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
      case ::ObstacleType::GOAL_POST:
        return ObstacleType::goalpost;
      case ::ObstacleType::UNKNOWN:
        return ObstacleType::unknown;
      case ::ObstacleType::ANONYMOUS_ROBOT:
        return ObstacleType::someRobot;
      case ::ObstacleType::HOSTILE_ROBOT:
        return ObstacleType::opponent;
      case ::ObstacleType::TEAM_ROBOT:
        return ObstacleType::teammate;
      case ::ObstacleType::FALLEN_ANONYMOUS_ROBOT:
        return ObstacleType::fallenSomeRobot;
      case ::ObstacleType::FALLEN_HOSTILE_ROBOT:
        return ObstacleType::fallenOpponent;
      case ::ObstacleType::FALLEN_TEAM_ROBOT:
        return ObstacleType::fallenTeammate;
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
