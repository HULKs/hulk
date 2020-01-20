#pragma once

#include <Framework/DataType.hpp>
#include <string>

#include "Tools/Math/Eigen.hpp"
#include "Tools/Math/Pose.hpp"
#include "Tools/Math/Velocity.hpp"

#include "Data/KickConfigurationData.hpp"

/**
 * Allows specification of different walking modes. Note that currently walking modes have
 * to be enabled in the MotionPlanner config, or all chosen modes will default to the PATH mode.
 *
 * \note When adding modes or changing their order, check if you have to adapt the remote controller
 * code.
 */
enum class WalkMode
{
  /**
   *  PATH our walking as usual: Walk to the specified target and avoid obstacles. Always try facing
   * the target position until near. If you don't know which mode to choose (which you should
   * know!), use this as default.
   */
  PATH,
  /**
   * Walk to the specified target and avoid obstacles, but immediately align according to the
   * orientation specified in the target. Be careful when using this, because it doesn't work well
   * with our current obstacle avoidance! Consider using DIRECT_WITH_ORIENTATION instead.
   */
  PATH_WITH_ORIENTATION,
  /// Directly walk to the specified target and ignore obstacles. Always try facing the target
  /// position until near.
  DIRECT,
  /// Walk to the specified target and ignore obstacles, but immediately align according to the
  /// orientation specified in the target.
  DIRECT_WITH_ORIENTATION,
  /// Walk to a pose from which a ball can be kicked without walking through the ball and without
  /// needing to walk too much sideways
  WALK_BEHIND_BALL,
  /// Similar to WALK_BEHIND_BALL, but prevent the robot from braking upon reaching the walk target
  /// near the ball, which is needed for dribbling
  DRIBBLE,
  /// Move *only* according to the specified velocity vector (target and obstacles will be ignored).
  VELOCITY
};

struct WalkData : public Uni::To, public Uni::From
{
  Pose target;
  InWalkKickType inWalkKickType = InWalkKickType::NONE;
  KickFoot kickFoot = KickFoot::NONE;
  WalkMode mode = WalkMode::PATH;
  /// Velocity specifications for walking (translation and rotation)
  Velocity velocity;

  void toValue(Uni::Value& value) const override
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["target"] << target;
    value["inWalkKickType"] << static_cast<int>(inWalkKickType);
    value["kickFoot"] << static_cast<int>(kickFoot);
    value["mode"] << static_cast<int>(mode);
    value["velocity"] << velocity;
  }

  void fromValue(const Uni::Value& value) override
  {
    int readNumber = 0;
    value["target"] >> target;
    value["inWalkKickType"] >> readNumber;
    inWalkKickType = static_cast<InWalkKickType>(readNumber);
    value["kickFoot"] >> readNumber;
    kickFoot = static_cast<KickFoot>(readNumber);
    value["mode"] >> readNumber;
    mode = static_cast<WalkMode>(readNumber);
    value["velocity"] >> velocity;
  }
};

struct WalkStopData
{
  bool gracefully;
};

struct KickData : public Uni::To, public Uni::From
{
  Vector2f ballDestination;
  Vector2f ballSource;
  KickType kickType = KickType::FORWARD;

  void toValue(Uni::Value& value) const override
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["ballDestination"] << ballDestination;
    value["ballSource"] << ballSource;
    value["kickType"] << static_cast<int>(kickType);
  }

  void fromValue(const Uni::Value& value) override
  {
    value["ballDestination"] >> ballDestination;
    value["ballSource"] >> ballSource;
    int readNumber = 0;
    value["kickType"] >> readNumber;
    kickType = static_cast<KickType>(readNumber);
  }
};

enum MotionJump
{
  NONE,
  SQUAT,
  TAKE_LEFT,
  TAKE_RIGHT,
  JUMP_LEFT,
  JUMP_RIGHT
};

struct JumpData
{
  MotionJump keep;
};

/** Containing data required to move the Head.
 * @author Finn Poppinga
 */
struct HeadAngleData : public Uni::To, public Uni::From
{
  float headYaw;
  float headPitch;
  float maxHeadYawVelocity;
  float maxHeadPitchVelocity;
  bool useEffectiveYawVelocity;

  void toValue(Uni::Value& value) const override
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["headYaw"] << headYaw;
    value["headPitch"] << headPitch;
    value["maxHeadYawVelocity"] << maxHeadYawVelocity;
    value["maxHeadPitchVelocity"] << maxHeadPitchVelocity;
    value["useEffectiveYawVelocity"] << useEffectiveYawVelocity;
  }

  void fromValue(const Uni::Value& value) override
  {
    value["headYaw"] >> headYaw;
    value["headPitch"] >> headPitch;
    value["maxHeadYawVelocity"] >> maxHeadYawVelocity;
    value["maxHeadPitchVelocity"] >> maxHeadPitchVelocity;
    value["useEffectiveYawVelocity"] >> useEffectiveYawVelocity;
  }
};

struct HeadLookAtData : public Uni::To, public Uni::From
{
  /// the target to look at in robot coordinates
  Vector3f targetPosition;
  float maxHeadYawVelocity;
  float maxHeadPitchVelocity;

  void toValue(Uni::Value& value) const override
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["targetPosition"] << targetPosition;
    value["maxHeadYawVelocity"] << maxHeadYawVelocity;
    value["maxHeadPitchVelocity"] << maxHeadPitchVelocity;
  }

  void fromValue(const Uni::Value& value) override
  {
    value["targetPosition"] >> targetPosition;
    value["maxHeadYawVelocity"] >> maxHeadYawVelocity;
    value["maxHeadPitchVelocity"] >> maxHeadPitchVelocity;
  }
};

struct PointData
{
  Vector3f relativePoint;
};

class MotionRequest : public DataType<MotionRequest>
{
public:
  /// the name of this DataType
  DataTypeName name = "MotionRequest";
  enum class BodyMotion
  {
    /// the stiffness of the body should be released
    DEAD = 0,
    /// the robot should stand in a pose from which it can directly start walking
    STAND,
    /// the robot should walk
    WALK,
    /// the robot should kick
    KICK,
    /// the robot should stand in an energy saving pose like during the penalized state
    PENALIZED,
    /// the robot should execute a jump motion
    JUMP,
    /// the robot should manage the fall. THIS MUST NOT HAVE A CORRESPONDING ACTION COMMAND
    FALL_MANAGER,
    /// the robot should stand up
    STAND_UP,
    /// the robot should sit down
    SIT_DOWN,
    /// the robot should sit up
    SIT_UP,
    /// the robot holds its angles at activation of the motion
    HOLD,
    /// the number of motions
    NUM
  };
  enum class ArmMotion
  {
    /// the arm should move with the body (is normally done implicitly)
    BODY,
    /// the arm should point to a point
    POINT
  };
  enum class HeadMotion
  {
    /// the head should move with the body (is normally done implicitly)
    BODY,
    /// head angles are passed directly
    ANGLES,
    /// the target to look at is passed and motion has to calculate the angles itself
    LOOK_AT
  };
  /// the motion that the body (legs + potentially arms + potentially head) should execute
  BodyMotion bodyMotion;
  /// the motion that the left arm should execute
  ArmMotion leftArmMotion;
  /// the motion that the right arm should execute
  ArmMotion rightArmMotion;
  /// the motion that the head should execute
  HeadMotion headMotion;
  /// the last walk data received
  WalkData walkData;
  /// the last walk stop data received
  WalkStopData walkStopData;
  /// the last kick data received
  KickData kickData;
  /// the last jump data received
  JumpData jumpData;
  /// the last head angle data received
  HeadAngleData headAngleData;
  /// the last head lookAt data received
  HeadLookAtData headLookAtData;
  /// the last point data received
  PointData pointData;
  /**
   * @brief reset sets the robot dead
   */
  void reset() override
  {
    bodyMotion = BodyMotion::DEAD;
    leftArmMotion = ArmMotion::BODY;
    rightArmMotion = ArmMotion::BODY;
    headMotion = HeadMotion::BODY;
  }
  /**
   * @brief usesArms indicates whether the body motion uses the arms in a way that they can't be
   * used independently
   * @return true iff the current motion uses the arms
   */
  bool usesArms() const
  {
    return bodyMotion == BodyMotion::DEAD || bodyMotion == BodyMotion::WALK ||
           bodyMotion == BodyMotion::KICK || bodyMotion == BodyMotion::PENALIZED ||
           bodyMotion == BodyMotion::JUMP || bodyMotion == BodyMotion::STAND_UP ||
           bodyMotion == BodyMotion::HOLD || bodyMotion == BodyMotion::SIT_DOWN ||
           bodyMotion == BodyMotion::SIT_UP;
  }
  /**
   * @brief usesHead indicates whether the body motion uses the head in a way that it can't be used
   * independently
   * @return true iff the current motion uses the head
   */
  bool usesHead() const
  {
    return bodyMotion == BodyMotion::DEAD || bodyMotion == BodyMotion::KICK ||
           bodyMotion == BodyMotion::PENALIZED || bodyMotion == BodyMotion::JUMP ||
           bodyMotion == BodyMotion::STAND_UP || bodyMotion == BodyMotion::HOLD;
  }

  void toValue(Uni::Value& value) const override
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["bodyMotion"] << static_cast<int>(bodyMotion);
    value["leftArmMotion"] << static_cast<int>(leftArmMotion);
    value["rightArmMotion"] << static_cast<int>(rightArmMotion);
    value["headMotion"] << static_cast<int>(headMotion);
    value["walkData"] << walkData;
    value["walkStopData"] << walkStopData.gracefully;
    value["kickData"] << kickData;
    value["jumpData"] << jumpData.keep;
    value["headAngleData"] << headAngleData;
    value["headLookAtData"] << headLookAtData;
    value["pointData"] << pointData.relativePoint;
  }

  void fromValue(const Uni::Value& value) override
  {
    int readNumber = 0;
    value["bodyMotion"] >> readNumber;
    bodyMotion = static_cast<BodyMotion>(readNumber);
    value["leftArmMotion"] >> readNumber;
    leftArmMotion = static_cast<ArmMotion>(readNumber);
    value["rightArmMotion"] >> readNumber;
    rightArmMotion = static_cast<ArmMotion>(readNumber);
    value["headMotion"] >> readNumber;
    headMotion = static_cast<HeadMotion>(readNumber);
    value["walkData"] >> walkData;
    value["walkStopData"] >> walkStopData.gracefully;
    value["kickData"] >> kickData;
    value["jumpData"] >> readNumber;
    jumpData.keep = static_cast<MotionJump>(readNumber);
    value["headAngleData"] >> headAngleData;
    value["headLookAtData"] >> headLookAtData;
    value["pointData"] >> pointData.relativePoint;
  }
};
