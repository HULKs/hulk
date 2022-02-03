#pragma once

#include <cstdint>
#include <stdexcept>
#include <string>
#include <vector>

#include "Framework/Configuration/Configuration.h"
#include "Framework/DataType.hpp"
#include "Tools/Math/Pose.hpp"


enum class Role
{
  PLAYER,
  DEMO,
  SHOOT_ON_HEAD_TOUCH
};

class PlayerConfiguration : public DataType<PlayerConfiguration>
{
public:
  /// the name of this DataType
  DataTypeName name__{"PlayerConfiguration"};
  /// the number of the team (in normal games this is YOUR_TEAM_NUMBER_HERE)
  unsigned int teamNumber = YOUR_TEAM_NUMBER_HERE;
  /// the number of the player
  unsigned int playerNumber = 0;
  /// the role of the player
  Role role = Role::PLAYER;
  /// port for SPL messages
  std::uint16_t port = 0;
  /// the x coordinates of the initial poses where the NAOs are placed (index is player number - 1)
  /// - the y coordinate is determined by the player number
  std::vector<float> initialPoses;

  void reset() override {}

  void toValue(Uni::Value& value) const override
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["teamNumber"] << teamNumber;
    value["playerNumber"] << playerNumber;
    value["role"] << static_cast<int>(role);
    value["port"] << port;
    value["initialPoses"] << initialPoses;
  }

  void fromValue(const Uni::Value& value) override
  {
    value["teamNumber"] >> teamNumber;
    value["playerNumber"] >> playerNumber;
    int numberRead = 0;
    value["role"] >> numberRead;
    role = static_cast<Role>(numberRead);
    uint32_t readPortNumber;
    value["port"] >> readPortNumber;
    port = static_cast<uint16_t>(readPortNumber);
    value["initialPoses"] >> initialPoses;
  }

  /**
   * @brief init loads the player configuration from a configuration file
   * @param config a reference to the configuration provider
   */
  void init(Configuration& config)
  {
    config.mount("Brain.Config", "Brain.json", ConfigurationType::HEAD);

    playerNumber = config.get("Brain.Config", "general.playerNumber").asInt32();
    if (playerNumber < 1)
    {
      throw std::runtime_error("Player number must not be < 1.");
    }
    teamNumber = config.get("Brain.Config", "general.teamNumber").asInt32();
    port = config.get("Brain.Config", "general.port").asInt32();

    std::string roleString = config.get("Brain.Config", "behavior.playerRole").asString();
    if (roleString == "player")
    {
      role = Role::PLAYER;
    }
    else if (roleString == "demo")
    {
      role = Role::DEMO;
    }
    else if (roleString == "shootOnHeadTouch")
    {
      role = Role::SHOOT_ON_HEAD_TOUCH;
    }
    else
    {
      throw std::runtime_error("The player role is something undefined.");
    }

    config.get("Brain.Config", "behavior.initialPoses") >> initialPoses;
  }
};
