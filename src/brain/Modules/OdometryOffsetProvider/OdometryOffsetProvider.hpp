#pragma once

#include "Data/OdometryData.hpp"
#include "Data/OdometryOffset.hpp"
#include "Framework/Module.hpp"
#include "Tools/Math/Pose.hpp"


class Brain;

class OdometryOffsetProvider : public Module<OdometryOffsetProvider, Brain>
{
public:
  /// the name of this module
  ModuleName name = "OdometryOffsetProvider";
  /**
   * @brief OdometryOffsetProvider initializes members
   * @param manager a reference to brain
   */
  OdometryOffsetProvider(const ModuleManagerInterface& manager);
  /**
   * @brief cycle updates the odometry offset
   */
  void cycle();

private:
  /// a reference to the odometry data
  const Dependency<OdometryData> odometryData_;
  /// a reference to the odometry offset
  Production<OdometryOffset> odometryOffset_;
  /// whether odometry data have already been used
  bool initialized_;
  /// the last odometry accumulator
  Pose lastOdometry_;
};
