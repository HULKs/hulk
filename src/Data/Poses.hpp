#pragma once

#include "Framework/Configuration/Configuration.h"
#include "Framework/DataType.hpp"
#include "Framework/Log/Log.hpp"
#include "Hardware/Definitions.hpp"
#include "Hardware/RobotInterface.hpp"
#include <fstream>
#include <map>

class Poses : public DataType<Poses>
{
public:
  /// the name of this DataType
  DataTypeName name__{"Poses"};

  enum class Type
  {
    PENALIZED,
    READY,
    SITTING,
    POSE_MAX
  };

  EnumArray<JointsArray<float>, Type, static_cast<std::size_t>(Type::POSE_MAX)> angles{};

  bool valid = false;

  /**
   * @brief reset resets members
   */
  void reset() override {}

  void toValue(Uni::Value& value) const override
  {
    value << angles;
  }

  void fromValue(const Uni::Value& value) override
  {
    value >> angles;
  }

  /**
   * @brief init loads the poses from configuration
   * @param config a reference to the configuration provider
   */
  void init(const RobotInterface& robotInterface)
  {
    auto loadPose = [this, &robotInterface](const Type type, const std::string& poseFile) {
      std::ifstream file{robotInterface.getFileRoot() + poseFile, std::ios::in};
      if (!file)
      {
        Log<M_TUHHSDK>(LogLevel::ERROR) << "Error producing Poses: could not open " << poseFile;
        throw std::runtime_error("Error producing Poses");
        return;
      }
      for (auto& angle : angles[type])
      {
        file >> angle;
      }
    };
    loadPose(Poses::Type::READY, "poses/Ready.pose");
    loadPose(Poses::Type::PENALIZED, "poses/Penalized.pose");
    loadPose(Poses::Type::SITTING, "poses/Sitting.pose");
    valid = true;
  }
};
