#pragma once

#include <cstdint>
#include <stdexcept>
#include <string>
#include <vector>

#include "Framework/DataType.hpp"
#include "Modules/Configuration/Configuration.h"
#include "Tools/Math/Pose.hpp"


enum class Role {
  PLAYER,
  DEMO,
  SHOOT_ON_HEAD_TOUCH
};

class PlayerConfiguration : public DataType<PlayerConfiguration> {
public:
  /// the name of this DataType
  DataTypeName name = "PlayerConfiguration";
  /// the number of the team (in normal games this is 24)
  unsigned int teamNumber = 24;
  /// the number of the player
  unsigned int playerNumber = 0;
  /// the role of the player
  Role role = Role::PLAYER;
  /// port for SPL messages
  std::uint16_t port = 0;
  /// the x coordinates of the initial poses where the NAOs are placed (index is player number - 1) - the y coordinate is determined by the player number
  std::vector<float> initialPoses;
  /// whether the robot is the transmitter robot in the NoWifiChallenge
  bool isNoWifiTransmitter = false;
  /// whether the robot is the receiver robot in the NoWifiChallenge
  bool isNoWifiReceiver = false;
  /**
   * @brief reset could reset members if it was necessary
   */
  void reset()
  {
  }

  virtual void toValue(Uni::Value& value) const
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["teamNumber"] << teamNumber;
    value["playerNumber"] << playerNumber;
    value["role"] << static_cast<int>(role);
    value["port"] << port;
    value["initialPoses"] << initialPoses;
    value["isNoWifiTransmitter"] << isNoWifiTransmitter;
    value["isNoWifiReceiver"] << isNoWifiReceiver;
  }

  virtual void fromValue(const Uni::Value& value)
  {
    value["teamNumber"] >> teamNumber;
    value["playerNumber"] >> playerNumber;
    int numberRead = 0;
    value["role"] >> numberRead;
    role = static_cast<Role>(numberRead);
    uint32_t readPortNumber;
    value["port"] >> readPortNumber;
    port = (uint16_t)readPortNumber;
    value["initialPoses"] >> initialPoses;
    value["isNoWifiTransmitter"] >> isNoWifiTransmitter;
    value["isNoWifiReceiver"] >> isNoWifiReceiver;
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

    isNoWifiTransmitter =
        config.hasProperty("Brain.Config", "challenges.isNoWifiTransmitter") && config.get("Brain.Config", "challenges.isNoWifiTransmitter").asBool();
    isNoWifiReceiver =
        config.hasProperty("Brain.Config", "challenges.isNoWifiReceiver") && config.get("Brain.Config", "challenges.isNoWifiReceiver").asBool();
  }
};
