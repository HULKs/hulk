#pragma once

#include "Framework/Module.hpp"

#include "Data/BodyDamageData.hpp"
#include "Data/ButtonData.hpp"
#include "Data/CycleInfo.hpp"
#include "Data/FootCollisionData.hpp"

class Brain;

/**
 * @brief This module detects collisions with the foot bumpers
 * A left, right, left or right, left, right foot bumper sequence is interpreted as a collision.
 */
class FootCollisionDetector : public Module<FootCollisionDetector, Brain>
{
public:
  /// the name of this module
  ModuleName name = "FootCollisionDetector";
  /**
   *@brief The constructor of this class
   */
  FootCollisionDetector(const ModuleManagerInterface& manager);

  void cycle() override;

private:
  /// Side of the foot bumper
  enum Side
  {
    LEFT,
    RIGHT,
    BOTH,
    NONE,
  };

  /// The collision state to track the collision sequence
  enum CollisionState
  {
    WAIT,
    TRIGGERED_ONCE,
    TRIGGERED_TWICE,
  };

  /**
   * @brief Evaluates collision switch from one foot to an other.
   * @return Whether there is a collision on the other foot
   */
  bool hasFootCollisionOnOtherFoot();

  /**
   * @brief Collects foot bumper states
   */
  void getFootBumperState();

  /**
   * @brief Keeps track of the current collision sequence
   */
  void updateCollisionState();

  /**
   * @brief Hold collision for a certain time, so that the obstacle does not disappear again
   * immediately
   */
  void holdCollision();

  /**
   * @brief Reset collision sequence after a while if it was accidentaly triggered
   */
  void resetCollisionState();

  /**
   * @brief Send debug information
   */
  void sendDebug();

  /// The side of last cycle detected foot bumper
  Side lastFootSide_;
  /// The side of the current detected foot bumper
  Side currentFootSide_;
  /// Timepoint when foot bumper sequence started
  TimePoint timeBumpSequenceBegin_;
  /// Timepoint of the current detected bumper
  TimePoint timeCurrentBumper_;
  /// Timepoint of the last cycle detedced bumper
  TimePoint timeLastCollision_;
  /// The state of current foot bumper squence
  CollisionState collisionState_;
  /// Time to hold collision state
  const Parameter<float> timeHoldState_;
  /// Time to hold collision
  const Parameter<float> timeHoldCollision_;
  /// a reference to the Buttonm Data to read foot bumper states
  const Dependency<ButtonData> buttonData_;
  /// a reference to the Cycle info. Used to calculate time since last collision
  const Dependency<CycleInfo> cycleInfo_;
  /// a reference to the body data provider to respect the hardware status of the foot bumper
  const Dependency<BodyDamageData> bodyDamageData_;
  ///  The collision data detected by foot bumpers
  Production<FootCollisionData> footCollisionData_;
};
