#pragma once

#include <string>

#include "Data/BodyPose.hpp"
#include "Data/CycleInfo.hpp"
#include "Data/GameControllerState.hpp"
#include "Data/PlayerConfiguration.hpp"
#include "Data/PlayingRoles.hpp"
#include "Data/RobotPosition.hpp"
#include "Data/StrikerAction.hpp"
#include "Data/TeamBallModel.hpp"
#include "Data/TeamPlayers.hpp"
#include "Data/TimeToReachBall.hpp"
#include "Framework/Module.hpp"


class Brain;

class PlayingRoleProvider : public Module<PlayingRoleProvider, Brain>
{
public:
  PlayingRoleProvider(const ModuleManagerInterface& manager);
  void cycle();

private:
  struct Player
  {
    Player(const unsigned int playerNumber, const float x)
      : playerNumber(playerNumber)
      , x(x)
    {
    }
    unsigned int playerNumber;
    float x;
  };

  const Parameter<bool> useTeamRole_;
  const Parameter<std::string> forceRole_;
  const Dependency<PlayerConfiguration> playerConfiguration_;
  const Dependency<RobotPosition> robotPosition_;
  const Dependency<TeamPlayers> teamPlayers_;
  const Dependency<TeamBallModel> teamBallModel_;
  const Dependency<GameControllerState> gameControllerState_;
  const Dependency<BodyPose> bodyPose_;
  const Dependency<StrikerAction> strikerAction_;
  const Dependency<CycleInfo> cycleInfo_;
  const Dependency<TimeToReachBall> timeToReachBall_;

  Production<PlayingRoles> playingRoles_;

  std::vector<PlayingRole> lastAssignment_;

  void selectRemainingPlayerRoles();
  void updateRole(const unsigned int playerNumber, const PlayingRole role);
  PlayingRole toRole(const std::string& configRole) const;
  float actualTimeToReachBall(const unsigned int playerNumber, const float timeToReachBall, const float timeToReachBallStriker) const;
  PlayingRole lastRoleOf(const unsigned int playerNumber) const;
};
