#pragma once

#include "Framework/DataType.hpp"
#include "Tools/Math/Eigen.hpp"
#include "Tools/Math/Pose.hpp"
#include "Tools/Math/Velocity.hpp"

#include "Data/HeadPositionData.hpp"
#include "Data/JumpOutput.hpp"
#include "Data/KickConfigurationData.hpp"


/**
 * @class ActionCommand represents the desired state of the robot
 */
class ActionCommand : public DataType<ActionCommand>
{
public:
  DataTypeName name__{"ActionCommand"};
  /**
   * @class Body contains the command for the body
   */
  class Body : public Uni::To, public Uni::From
  {
  public:
    enum class MotionType
    {
      /// the stiffness of the body should be released
      DEAD,
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
      /// the robot is in puppet mode
      PUPPET,
      /// the number of motions
      MAX
    };
    template <typename T>
    using MotionTypeArray =
        EnumArray<T, ActionCommand::Body::MotionType,
                  static_cast<std::size_t>(ActionCommand::Body::MotionType::MAX)>;

    /// Allows specification of different walking modes.
    enum class WalkMode
    {
      /// Walk to the specified target and avoid obstacles. Always face the target position until
      /// the robot is close to the target pose
      PATH,
      /// Walk to the specified target and avoid obstacles, but immediately align according to the
      /// orientation specified in the target pose
      PATH_WITH_ORIENTATION,
      /// Walk to the specified target, ignoring obstacles. Always face the target position until
      /// the robot is close to the target.
      DIRECT,
      /// Walk to the specified target, ignoring obstacles, but immediately align according to the
      /// orientation specified in the target.
      DIRECT_WITH_ORIENTATION,
      /// Walk to a pose from which a ball can be kicked without walking through the ball and
      /// without needing to walk too much sideways
      WALK_BEHIND_BALL,
      /// Similar to WALK_BEHIND_BALL, but prevent the robot from braking upon reaching the walk
      /// target near the ball, which is needed for dribbling
      DRIBBLE,
      /// Move according to the specified velocity vector
      VELOCITY
    };
    /// the requested body motion type
    MotionType type = MotionType::DEAD;
    /// the target of a walk command
    Pose walkTarget;
    /// specifies the walk mode for the motion planner
    WalkMode walkMode = WalkMode::PATH;
    /// velocity parameter used by the motion planner
    Velocity walkVelocity;
    /// the ball position for a kick command
    Vector2f ballPosition = Vector2f::Zero();
    /// the target ball position for a kick command
    Vector2f ballTarget = Vector2f::Zero();
    /// the KickType of a kick command
    KickType kickType = KickType::NONE;
    /// the type of the in walk kick
    InWalkKickType inWalkKickType = InWalkKickType::NONE;
    /// the foot used for in walk kicking
    KickFoot kickFoot = KickFoot::NONE;
    /// the jump type for a jump command
    JumpOutput::Type jumpType = JumpOutput::Type::NONE;

    /**
     * @brief dead creates a dead action command for the body
     * @return a dead action command for the body
     */
    static Body dead()
    {
      Body body;
      body.type = MotionType::DEAD;
      return body;
    }
    /**
     * @brief stand creates a stand action command for the body
     * @return a stand action command for the body
     */
    static Body stand()
    {
      Body body;
      body.type = MotionType::STAND;
      return body;
    }
    /**
     * @brief walk creates a walk action command for the body
     * @param walkTarget the (relative) pose where the robot should go
     * @param walkingMode specifies the mode of operation for the path- and step-planner
     * @param velocity desired walking velocities, movement and rotation. [m/s]
     * @param inWalkKickType the type of the in walk kick
     * @param ballTarget absolute field coordinates specifying the desired destination for the ball
     * @return a walk action command for the body
     */
    static Body walk(const Pose& walkTarget, const WalkMode walkMode = WalkMode::PATH,
                     const Velocity& velocity = Velocity(),
                     const InWalkKickType inWalkKickType = InWalkKickType::NONE,
                     const KickFoot kickFoot = KickFoot::NONE,
                     const Vector2f& ballTarget = Vector2f::Zero())
    {
      Body body;
      body.type = MotionType::WALK;
      body.walkTarget = walkTarget;
      body.walkMode = walkMode;
      body.walkVelocity = velocity;
      body.inWalkKickType = inWalkKickType;
      body.kickFoot = kickFoot;
      body.ballTarget = ballTarget;
      return body;
    }
    /**
     * @brief kick creates a kick action command for the body
     * @param ballPosition the (relative) position where the kick should assume the ball to be
     * @param ballTarget the (relative) position where the ball should end up
     * @param kickType the type of kick
     * @return a kick action command for the body
     */
    static Body kick(const Vector2f& ballPosition, const Vector2f& ballTarget,
                     const KickType kickType)
    {
      Body body;
      body.type = MotionType::KICK;
      body.ballPosition = ballPosition;
      body.ballTarget = ballTarget;
      body.kickType = kickType;
      return body;
    }
    /**
     * @brief penalized creates a penalized action command for the body
     * @return a penalized action command for the body
     */
    static Body penalized()
    {
      Body body;
      body.type = MotionType::PENALIZED;
      return body;
    }
    /**
     * @brief jump creates a jump action command for the body
     * @param jumpType the type of the jump motion
     * @return a jump action command for the body
     */
    static Body jump(const JumpOutput::Type jumpType)
    {
      Body body;
      body.type = MotionType::JUMP;
      body.jumpType = jumpType;
      return body;
    }
    /**
     * @brief standUp creates a stand up action command for the body
     * @return a stand up action command for the body
     */
    static Body standUp()
    {
      Body body;
      body.type = MotionType::STAND_UP;
      return body;
    }
    /**
     * @brief sitDown creates a sit down action command for the body
     * @return a sit down action command for the body
     */
    static Body sitDown()
    {
      Body body;
      body.type = MotionType::SIT_DOWN;
      return body;
    }
    /**
     * @brief sitUp creates a sit up action command for the body
     * @return a sit up action command for the body
     */
    static Body sitUp()
    {
      Body body;
      body.type = MotionType::SIT_UP;
      return body;
    }
    /**
     * @brief hold creates a hold action command for the body
     * @return a hold action command for the body
     */
    static Body hold()
    {
      Body body;
      body.type = MotionType::HOLD;
      return body;
    }
    /**
     * @brief hold creates a puppet action command for the body
     * @return a hold action command for the body
     */
    static Body puppet()
    {
      Body body;
      body.type = MotionType::PUPPET;
      return body;
    }
    /**
     * @brief usesArms indicates whether the body motion uses the arms in a way that they can't be
     * used independently
     * @return true iff the current motion uses the arms
     */
    bool usesArms() const
    {
      return type == MotionType::DEAD || type == MotionType::WALK || type == MotionType::KICK ||
             type == MotionType::PENALIZED || type == MotionType::JUMP ||
             type == MotionType::STAND_UP || type == MotionType::HOLD ||
             type == MotionType::SIT_DOWN || type == MotionType::SIT_UP;
    }
    /**
     * @brief usesHead indicates whether the body motion uses the head in a way that it can't be
     * used independently
     * @return true iff the current motion uses the head
     */
    bool usesHead() const
    {
      return type == MotionType::DEAD || type == MotionType::KICK ||
             type == MotionType::PENALIZED || type == MotionType::JUMP ||
             type == MotionType::STAND_UP || type == MotionType::HOLD;
    }

    void toValue(Uni::Value& value) const override
    {
      value = Uni::Value(Uni::ValueType::OBJECT);
      value["type"] << static_cast<unsigned int>(type);
      value["walkTarget"] << walkTarget;
      value["walkMode"] << static_cast<unsigned int>(walkMode);
      value["velocity"] << walkVelocity;
      value["ballPosition"] << ballPosition;
      value["ballTarget"] << ballTarget;
      value["kickType"] << static_cast<unsigned int>(kickType);
      value["inWalkKickType"] << static_cast<unsigned int>(inWalkKickType);
      value["kickFoot"] << static_cast<unsigned int>(kickFoot);
      value["jumpType"] << static_cast<unsigned int>(jumpType);
    }

    void fromValue(const Uni::Value& value) override
    {
      unsigned int enumValue = 0;
      value["type"] >> enumValue;
      type = static_cast<MotionType>(enumValue);
      value["walkTarget"] >> walkTarget;
      value["walkMode"] >> enumValue;
      walkMode = static_cast<WalkMode>(enumValue);
      value["velocity"] >> walkVelocity;
      value["ballPosition"] >> ballPosition;
      value["ballTarget"] >> ballTarget;
      value["kickType"] >> enumValue;
      kickType = static_cast<KickType>(enumValue);
      value["inWalkKickType"] >> enumValue;
      inWalkKickType = static_cast<InWalkKickType>(enumValue);
      value["kickFoot"] >> enumValue;
      kickFoot = static_cast<KickFoot>(enumValue);
      value["jumpType"] >> enumValue;
      jumpType = static_cast<JumpOutput::Type>(enumValue);
    }

  private:
    /**
     * @brief Body creates an undefined body action command
     */
    Body() = default;
    friend class ActionCommand;
  };
  /**
   * @class Arm contains the command for an arm
   */
  class Arm : public Uni::To, public Uni::From
  {
  public:
    enum class MotionType
    {
      /// the arm should move with the body (is normally done implicitly)
      BODY,
      /// the arm should point to a point
      POINT
    };
    /// the requested arm motion type
    MotionType type = MotionType::BODY;
    /// the (relative) point where the arm should point to
    Vector3f target = Vector3f::Zero();

    /**
     * @brief body creates a body action command for an arm
     * @return a body action command for an arm
     */
    static Arm body()
    {
      Arm arm;
      arm.type = MotionType::BODY;
      return arm;
    }
    /**
     * @brief point creates a point action command for an arm
     * @param target the point where the arm should point to
     * @return a point action command for an arm
     */
    static Arm point(const Vector3f& target)
    {
      Arm arm;
      arm.type = MotionType::POINT;
      arm.target = target;
      return arm;
    }

    void toValue(Uni::Value& value) const override
    {
      value = Uni::Value(Uni::ValueType::OBJECT);
      value["type"] << static_cast<unsigned int>(type);
      value["target"] << target;
    }

    void fromValue(const Uni::Value& value) override
    {
      unsigned int enumValue = 0;
      value["type"] >> enumValue;
      type = static_cast<MotionType>(enumValue);
      value["target"] >> target;
    }

  private:
    /**
     * @brief Arm creates an undefined arm action command
     */
    Arm() = default;
    friend class ActionCommand;
  };
  /**
   * @class Head contains the command for the head
   */
  class Head : public Uni::To, public Uni::From
  {
  public:
    enum class MotionType
    {
      /// the head should move with the body (is normally done implicitly)
      BODY,
      /// head angles are passed directly
      ANGLES,
      /// the target to look at is passed and motion has to calculate the angles itself
      LOOK_AT
    };
    /// the requested head motion type
    MotionType type = MotionType::BODY;
    /// the desired yaw angle
    float yaw = 0.f;
    /// the desired pitch angle
    float pitch = 0.f;
    /// the target to look at (in robot coordinates)
    Vector3f targetPosition = Vector3f::Zero();
    /// the maximal angular velocity of the yaw joint to reach the target
    float maxYawVelocity = 0.f;
    /// the maximal angular velocity of the pitch joint to reach the target
    float maxPitchVelocity = 0.f;
    /// true if effective velocity is to be requested (heads will move with requested velocity
    /// relative to ground)
    bool useEffectiveYawVelocity = true;

    /**
     * @brief body creates a body action command for the head
     * @return a body action command for the head
     */
    static Head body()
    {
      Head head;
      head.type = MotionType::BODY;
      return head;
    }
    /**
     * @brief angles creates an angles action command for the head
     * @param yaw the desired yaw angle
     * @param pitch the desired pitch angle
     * @param yawVelocity the maximal angular velocity of the yaw joint to reach the target (zero
     * means maximal possible velocity)
     * @param useEffectiveYawVelocity set to true if the yaw velocity is to be achieved with
     * respect to ground
     * @param pitchVelocity the maximal angular velocity of the pitch joint to reach the target
     * (zero means maximal possible velocity)
     * @return an angles action command for the head
     */
    static Head angles(const float yaw = 0, const float pitch = 0, const float maxYawVelocity = 0,
                       const float maxPitchVelocity = 0, const bool useEffectiveYawVelocity = true)
    {
      Head head;
      head.type = MotionType::ANGLES;
      head.yaw = yaw;
      head.pitch = pitch;
      head.maxYawVelocity = maxYawVelocity;
      head.useEffectiveYawVelocity = useEffectiveYawVelocity;
      head.maxPitchVelocity = maxPitchVelocity;
      return head;
    }

    /**
     * @brief angles creates an angles action command for the head
     * @param headPosition the desired head position
     * @param yawvelocity the maximal angular velocity of the yaw joint to reach the target (zero
     * means maximal possible velocity)
     * @param pitchvelocity the maximal angular velocity of the pitch joint to reach the target
     * (zero means maximal possible velocity)
     * @return an angles action command for the head
     */
    static Head angles(const HeadPosition& headPosition, const float maxYawVelocity = 0.f,
                       const float maxPitchVelocity = 0.f,
                       const bool useEffectiveYawVelocity = true)
    {
      Head head;
      head.type = MotionType::ANGLES;
      head.yaw = headPosition.yaw;
      head.pitch = headPosition.pitch;
      head.maxYawVelocity = maxYawVelocity;
      head.useEffectiveYawVelocity = useEffectiveYawVelocity;
      head.maxPitchVelocity = maxPitchVelocity;
      return head;
    }
    /**
     * @brief lookAt creates a lookAt action command for the head
     * @param targetPosition the target position in robot coordinates
     * @param yawVelocity the maximal angular velocity of the yaw joint to reach the target (zero
     * means maximal possible velocity)
     * @param pitchVelocity the maximal angular velocity of the pitch joint to reach the target
     * (zero means maximal possible velocity)
     * @return a lookAt action command for the head
     */
    static Head lookAt(const Vector3f& targetPosition, const float maxYawVelocity = 0.f,
                       const float maxPitchVelocity = 0.f)
    {
      Head head;
      head.type = MotionType::LOOK_AT;
      head.maxYawVelocity = maxYawVelocity;
      head.useEffectiveYawVelocity = false;
      head.maxPitchVelocity = maxPitchVelocity;
      head.targetPosition = targetPosition;
      return head;
    }

    void toValue(Uni::Value& value) const override
    {
      value = Uni::Value(Uni::ValueType::OBJECT);
      value["type"] << static_cast<unsigned int>(type);
      value["yaw"] << yaw;
      value["pitch"] << pitch;
      value["targetPosition"] << targetPosition;
      value["maxYawVelocity"] << maxYawVelocity;
      value["maxPitchVelocity"] << maxPitchVelocity;
      value["useEffectiveYawVelocity"] << useEffectiveYawVelocity;
    }

    void fromValue(const Uni::Value& value) override
    {
      unsigned int enumValue = 0;
      value["type"] >> enumValue;
      type = static_cast<MotionType>(enumValue);
      value["yaw"] >> yaw;
      value["pitch"] >> pitch;
      value["targetPosition"] >> targetPosition;
      value["maxYawVelocity"] >> maxYawVelocity;
      value["maxPitchVelocity"] >> maxPitchVelocity;
      value["useEffectiveYawVelocity"] >> useEffectiveYawVelocity;
    }

  private:
    /**
     * @brief Head creates an undefined head action command
     */
    Head() = default;
    friend class ActionCommand;
  };
  /**
   * @class LED contains the command for an LED
   */
  class LED : public Uni::To, public Uni::From
  {
  public:
    /**
     * @brief Modes that can be applied to single eyes.
     */
    enum class EyeMode
    {
      OFF,
      COLOR,
      RAINBOW
    };
    /// The eye mode
    EyeMode eyeMode = EyeMode::OFF;
    /// the red intensity in [0,1]
    float r = 0.f;
    /// the green intensity in [0,1]
    float g = 0.f;
    /// the blue intensity in [0,1]
    float b = 0.f;
    /**
     * @brief colors creates a colors action command for an LED
     * @param r the red intensity in [0,1]
     * @param g the green intensity in [0,1]
     * @param b the blue intensity in [0,1]
     * @return a colors action command for an LED
     */
    static LED colors(const float r = 0, const float g = 0, const float b = 0)
    {
      LED led;
      led.eyeMode = EyeMode::COLOR;
      led.r = r;
      led.g = g;
      led.b = b;
      return led;
    }
    /**
     * @brief off creates an off action command for an LED
     * @return an off action command for an LED
     */
    static LED off()
    {
      LED led;
      led.eyeMode = EyeMode::OFF;
      return led;
    }
    /**
     * @brief white creates a white action command for an LED
     * @return a white action command for an LED
     */
    static LED white()
    {
      return colors(1, 1, 1);
    }
    /**
     * @brief green creates a green action command for an LED
     * @return a green action command for an LED
     */
    static LED green()
    {
      return colors(0, 1, 0);
    }
    /**
     * @brief yellow creates a yellow action command for an LED
     * @return a yellow action command for an LED
     */
    static LED yellow()
    {
      return colors(1, 1, 0);
    }
    /**
     * @brief red creates a red action command for an LED
     * @return a red action command for an LED
     */
    static LED red()
    {
      return colors(1, 0, 0);
    }
    /**
     * @brief blue creates a blue action command for an LED
     * @return a blue action command for an LED
     */
    static LED blue()
    {
      return colors(0, 0, 1);
    }
    /**
     * @brief lightblue creates a lightblue action command for an LED
     * @return a lightblue action command for an LED
     */
    static LED lightblue()
    {
      return colors(0, 1, 1);
    }
    /**
     * @brief pink creates a pink action command for an LED
     * @return a pink action command for an LED
     */
    static LED pink()
    {
      return colors(1, 0.07f, 0.58f);
    }
    /**
     * @brief raspberry creates a raspberry action command for an LED
     * @return a raspberry action command for an LED
     */
    static LED raspberry()
    {
      return colors(1, 0, 0.5);
    }
    /**
     * @brief violet creates a violet action command for an LED
     * @return a violet action command for an LED
     */
    static LED violet()
    {
      return colors(0.5, 0, 1);
    }

    static LED rainbow()
    {
      LED led;
      led.eyeMode = EyeMode::RAINBOW;
      return led;
    }

    void toValue(Uni::Value& value) const override
    {
      value = Uni::Value(Uni::ValueType::OBJECT);
      value["eyeMode"] << static_cast<unsigned int>(eyeMode);
      value["r"] << r;
      value["g"] << g;
      value["b"] << b;
    }

    void fromValue(const Uni::Value& value) override
    {
      unsigned int enumValue{0};
      value["eyeMode"] >> enumValue;
      eyeMode = static_cast<EyeMode>(enumValue);
      value["r"] >> r;
      value["g"] >> g;
      value["b"] >> b;
    }

  private:
    /**
     * @brief LED creates an undefined LED action command
     */
    LED() = default;
    friend class ActionCommand;
  };
  /**
   * @brief dead creates a dead action command
   * @return a dead action command
   */
  static ActionCommand dead()
  {
    return ActionCommand{Body::dead()};
  }
  /**
   * @brief stand creates a stand action command
   * @return a stand action command
   */
  static ActionCommand stand()
  {
    return ActionCommand{Body::stand(), Head::angles()};
  }
  /**
   * @brief walk creates a walk action command
   * @param walkTarget the (relative) pose where the robot should go
   * @param walkingMode specifies the mode of operation for the motionplanner like following path
   * with fixed orientation
   * @param velocity desired walking velocities, movement and rotation [percentage of max speed]
   * @param inWalkKickType the type of the in walk kick
   * @param kickFoot the foot used for kicking
   * @param ballTarget absolute field coordinates specifying the desired destination for the ball
   * @return a walk action command
   */
  static ActionCommand walk(const Pose& walkTarget,
                            const Body::WalkMode walkMode = Body::WalkMode::PATH,
                            const Velocity& velocity = Velocity(),
                            const InWalkKickType inWalkKickType = InWalkKickType::NONE,
                            const KickFoot kickFoot = KickFoot::NONE,
                            const Vector2f& ballTarget = Vector2f::Zero())
  {
    // Target pose should not be nan
    assert(!std::isnan(walkTarget.x()) && "walkTarget pose.x is nan");
    assert(!std::isnan(walkTarget.y()) && "walkTarget pose.y is nan");
    assert(!std::isnan(walkTarget.angle()) && "walkTarget pose.angle is nan");

    return ActionCommand{
        Body::walk(walkTarget, walkMode, velocity, inWalkKickType, kickFoot, ballTarget)};
  }
  /**
   * @brief walkVelocity creates an action command for walking according to the specified velocity,
   * which contains direction and speed
   * @param velocity defines the translation direction and velocity as well as rotation velocity for
   * walking [percentage of max speed]
   * @param inWalkKickType the type of the in walk kick, set to NONE if no kick is to be performed
   * @param kickFoot the foot used for kicking
   * return a walk action command for the body using the velocity walking mode
   */
  static ActionCommand walkVelocity(const Velocity& velocity,
                                    const InWalkKickType inWalkKickType = InWalkKickType::NONE,
                                    const KickFoot kickFoot = KickFoot::NONE)
  {
    // Use an empty pose for the walkTarget because it will be ignored in velocity mode
    return ActionCommand{
        Body::walk(Pose(), Body::WalkMode::VELOCITY, velocity, inWalkKickType, kickFoot)};
  }
  /**
   * @brief kick creates a kick action command
   * @param ballPosition the (relative) position where the kick should assume the ball to be
   * @param ballTarget the (relative) position where the ball should end up
   * @param kickType the type of kick
   * @return a kick action command
   */
  static ActionCommand kick(const Vector2f& ballPosition, const Vector2f& ballTarget,
                            const KickType kickType)
  {
    return ActionCommand{Body::kick(ballPosition, ballTarget, kickType)};
  }
  /**
   * @brief penalized creates a penalized action command
   * @return a penalized action command
   */
  static ActionCommand penalized()
  {
    return ActionCommand{Body::penalized()};
  }
  /**
   * @brief jump creates a jump action command
   * @param jump the type of the jump motion
   * @return a jump action command
   */
  static ActionCommand jump(const JumpOutput::Type jumpType)
  {
    return ActionCommand{Body::jump(jumpType)};
  }
  /**
   * @brief standUp creates a stand up action command
   * @return a stand up action command
   */
  static ActionCommand standUp()
  {
    return ActionCommand{Body::standUp()};
  }
  /**
   * @brief sitDown creates a sit down action command
   * @return a sit down action command
   */
  static ActionCommand sitDown()
  {
    return ActionCommand{Body::sitDown()};
  }
  /**
   * @brief sitUp creates a sit up action command
   * @return a sit up action command
   */
  static ActionCommand sitUp()
  {
    return ActionCommand{Body::sitUp()};
  }
  /**
   * @brief hold creates a hold action command
   * @return a hold action command
   */
  static ActionCommand hold()
  {
    return ActionCommand{Body::hold()};
  }
  /**
   * @brief hold creates a hold action command
   * @return a hold action command
   */
  static ActionCommand puppet()
  {
    return ActionCommand{Body::puppet()};
  }
  /**
   * @brief combineBody replaces the body part of an action command
   * @param body the new body part of the action command
   * @return reference to this
   */
  ActionCommand& combineBody(const Body& body)
  {
    body_ = body;
    return *this;
  }
  /**
   * @brief combineLeftArm replaces the left arm part of an action command
   * @param left_arm the new left arm part of the action command
   * @return reference to this
   */
  ActionCommand& combineLeftArm(const Arm& leftArm)
  {
    leftArm_ = leftArm;
    return *this;
  }
  /**
   * @brief combineRightArm replaces the right arm part of an action command
   * @param right_arm the new right arm part of the action command
   * @return reference to this
   */
  ActionCommand& combineRightArm(const Arm& rightArm)
  {
    rightArm_ = rightArm;
    return *this;
  }
  /**
   * @brief combineHead replaces the head part of an action command
   * @param head the new head part of the action command
   * @return reference to this
   */
  ActionCommand& combineHead(const Head& head)
  {
    head_ = head;
    return *this;
  }
  /**
   * @brief combineLeftLED replaces the left LED part of an action command
   * @param left_led the new left LED part of the action command
   * @return reference to this
   */
  ActionCommand& combineLeftLED(const LED& leftLED)
  {
    leftLED_ = leftLED;
    return *this;
  }
  /**
   * @brief combineRightLED replaces the right LED part of an action command
   * @param right_led the new right LED part of the action command
   * @return reference to this
   */
  ActionCommand& combineRightLED(const LED& rightLED)
  {
    rightLED_ = rightLED;
    return *this;
  }
  /**
   * @brief body returns the body part of the command
   * @return the body part of the command
   */
  const Body& body() const
  {
    return body_;
  }
  /**
   * @brief leftArm returns the left arm part of the command
   * @return the left arm part of the command
   */
  const Arm& leftArm() const
  {
    return leftArm_;
  }
  /**
   * @brief rightArm returns the right arm part of the command
   * @return the right arm part of the command
   */
  const Arm& rightArm() const
  {
    return rightArm_;
  }
  /**
   * @brief head returns the head part of the command
   * @return the head part of the command
   */
  const Head& head() const
  {
    return head_;
  }
  /**
   * @brief leftLED returns the left LED part of the command
   * @return the left LED part of the command
   */
  const LED& leftLED() const
  {
    return leftLED_;
  }
  /**
   * @brief rightLED returns the right LED part of the command
   * @return the right LED part of the command
   */
  const LED& rightLED() const
  {
    return rightLED_;
  }

  void reset() override
  {
    body_ = Body::dead();
    leftArm_ = Arm::body();
    rightArm_ = Arm::body();
    head_ = Head::body();
    leftLED_ = LED::off();
    rightLED_ = LED::off();
    valid_ = false;
  }

  void toValue(Uni::Value& value) const override
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["body"] << body_;
    value["head"] << head_;
    value["leftArm"] << leftArm_;
    value["rightArm"] << rightArm_;
    value["leftLED"] << leftLED_;
    value["rightLED"] << rightLED_;
    value["valid"] << valid_;
  }

  void fromValue(const Uni::Value& value) override
  {
    value["body"] >> body_;
    value["head"] >> head_;
    value["leftArm"] >> leftArm_;
    value["rightArm"] >> rightArm_;
    value["leftLED"] >> leftLED_;
    value["rightLED"] >> rightLED_;
    value["valid"] >> valid_;
  }

  /**
   * @brief default constructor
   */
  ActionCommand() = default;

private:
  /**
   * @brief ActionCommand creates an action command from commands for every part
   * @param body the command for the body
   * @param leftArm the command for the left arm
   * @param rightArm the command for the right arm
   * @param head the command for the head
   * @param leftLED the command for the left LED
   * @param rightLED the command for the right LED
   */
  explicit ActionCommand(Body body, Head head = Head::body(), Arm leftArm = Arm::body(),
                         Arm rightArm = Arm::body(), LED leftLED = LED::off(),
                         LED rightLED = LED::off())
    : body_(std::move(body))
    , head_(std::move(head))
    , leftArm_(std::move(leftArm))
    , rightArm_(std::move(rightArm))
    , leftLED_(std::move(leftLED))
    , rightLED_(std::move(rightLED))
  {
  }
  /// the command for the body
  Body body_;
  /// the command for the head
  Head head_;
  /// the command for the left arm
  Arm leftArm_;
  /// the command for the right arm
  Arm rightArm_;
  /// the command for the left eye LED
  LED leftLED_;
  /// the command for the right eye LED
  LED rightLED_;
  /// whether the action command is valid
  bool valid_ = false;
};
