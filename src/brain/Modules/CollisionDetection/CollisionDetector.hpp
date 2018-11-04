#pragma once

#include "Framework/Module.hpp"
#include "Data/BodyDamageData.hpp"
#include "Data/CollisionDetectorData.hpp"
#include "Data/CycleInfo.hpp"
#include "Data/JointSensorData.hpp"
#include "Data/MotionRequest.hpp"
#include "Data/TeamObstacleData.hpp"
#include "Tools/Time.hpp"


class Brain;

class CollisionDetector : public Module<CollisionDetector, Brain>
{
public:
  ModuleName name = "CollisionDetector";
  CollisionDetector(const ModuleManagerInterface& manager);
  void cycle();


private:
  enum Side
  {
    SIDE_LEFT = 0,
    SIDE_RIGHT = 1
  };
  /**
   * @brief predictCollisionsFromObstacles predicts collisions from obstacles provided by the
   * obstacle filter
   */
  void predictCollisionsFromObstacles();
  /**
   * @brief detectCollisionsFromArmCurrents detects collisions from patterns in the measured arm
   * current
   */
  void updateOutput();
  /**
   * @brief sendDebug sends the debug data via the debug protocol and enables logging if required
   */
  void sendDebug() const;

  /// Counter to iterate the lastStates
  int bufferIter_;
  /// Time of last detection ( used to retain state longer), [left, right]
  std::array<TimePoint, 2> timeOfLastDetection_;
  /// State of last N cycles (N, config parameter)
  std::vector<std::vector<bool>> lastStates_;

  /// Time to hold collision state
  Parameter<float> timeHoldState_;
  /// All obstacles will get ignored if beyond this distance, in meter
  Parameter<float> obstacleRangeOfVision_;
  /// some safety distance to actually predict, not only detect collisions
  Parameter<float> collisionSafetyDistance_;
  /// Size of buffer, size of majority quorum
  Parameter<int> sizeOfBuffer_;
  /// The teamObstacleData gives access to all currently known obstacles (local model and obstacles of the team)
  const Dependency<TeamObstacleData> teamObstacleData_;
  /// needed to avoid bad readings from electric current at damaged parts
  /// the game controller state is used to avoid collision output in SET
  const Dependency<GameControllerState> gameControllerState_;
  /// a reference to the Cycle info. Used to calculate time since last collision
  const Dependency<CycleInfo> cycleInfo_;

  /// Producing collisionData
  Production<CollisionDetectorData> collisionDetectorData_;
};
