#pragma once

#include "Data/EyeLEDRequest.hpp"
#include "Data/MotionRequest.hpp"
#include "Tools/Math/Eigen.hpp"
#include "Tools/Math/Pose.hpp"


/**
 * @class ActionCommand represents the desired state of the robot
 */
class ActionCommand
{
public:
  /**
   * @class Body contains the command for the body
   */
  class Body
  {
  public:
    /**
     * @brief dead creates a dead action command for the body
     * @return a dead action command for the body
     */
    static Body dead()
    {
      Body body;
      body.type_ = MotionRequest::BodyMotion::DEAD;
      return body;
    }
    /**
     * @brief stand creates a stand action command for the body
     * @return a stand action command for the body
     */
    static Body stand()
    {
      Body body;
      body.type_ = MotionRequest::BodyMotion::STAND;
      return body;
    }
    /**
     * @brief walk creates a walk action command for the body
     * @param target the (relative) pose where the robot should go
     * @param walkingMode specifies the mode of operation for the motionplanner like following path with fixed orientation
     * @param velocity Desired walking velocities, movement and rotation. [m/s]
     * @param inWalkKickType the type of the in walk kick
     * @return a walk action command for the body
     */
    static Body walk(const Pose& target, const WalkMode walkingMode = WalkMode::PATH, const Velocity& velocity = Velocity(),
                     const InWalkKickType inWalkKickType = InWalkKickType::NONE)
    {
      Body body;
      body.type_ = MotionRequest::BodyMotion::WALK;
      body.target_ = target;
      body.walkingMode_ = walkingMode;
      body.velocity_ = velocity;
      body.inWalkKickType_ = inWalkKickType;
      return body;
    }
    /**
     * @brief kick creates a kick action command for the body
     * @param ball_position the (relative) position where the kick should assume the ball to be
     * @param ball_target the (relative) position where the ball should end up
     * @param kickType the type of kick
     * @return a kick action command for the body
     */
    static Body kick(const Vector2f& ball_position, const Vector2f& ball_target, const KickType kickType)
    {
      Body body;
      body.type_ = MotionRequest::BodyMotion::KICK;
      body.ballPosition_ = ball_position;
      body.ballTarget_ = ball_target;
      body.kickType_ = kickType;
      return body;
    }
    /**
     * @brief penalized creates a penalized action command for the body
     * @return a penalized action command for the body
     */
    static Body penalized()
    {
      Body body;
      body.type_ = MotionRequest::BodyMotion::PENALIZED;
      return body;
    }
    /**
     * @brief keeper creates a keeper action command for the body
     * @param keeper_type the type of the keeper motion
     * @return a keeper action command for the body
     */
    static Body keeper(const MotionKeeper keeper_type)
    {
      Body body;
      body.type_ = MotionRequest::BodyMotion::KEEPER;
      body.keeperType_ = keeper_type;
      return body;
    }
    /**
     * @brief standUp creates a stand up action command for the body
     * @return a stand up action command for the body
     */
    static Body standUp()
    {
      Body body;
      body.type_ = MotionRequest::BodyMotion::STAND_UP;
      return body;
    }
    /**
     * @brief hold creates a hold action command for the body
     * @return a hold action command for the body
     */
    static Body hold()
    {
      Body body;
      body.type_ = MotionRequest::BodyMotion::HOLD;
      return body;
    }
    /**
     * @brief type returns the type of the command
     * @return the type of the command
     */
    MotionRequest::BodyMotion type() const
    {
      return type_;
    }

  private:
    /**
     * @brief Body creates an undefined body action command
     */
    Body() = default;
    /// the requested body motion type
    MotionRequest::BodyMotion type_;
    /// the target of a walk command
    Pose target_;
    /// specifies the walking mode for the motion planner
    WalkMode walkingMode_;
    /// velocity parameter used by the motion planner
    Velocity velocity_;
    /// the ball position for a kick command
    Vector2f ballPosition_;
    /// the target ball position for a kick command
    Vector2f ballTarget_;
    /// the KickType of a kick command
    KickType kickType_;
    /// the type of the in walk kick
    InWalkKickType inWalkKickType_;
    /// the keeper type for a keeper command
    MotionKeeper keeperType_;
    friend class ActionCommand;
  };
  /**
   * @class Arm contains the command for an arm
   */
  class Arm
  {
  public:
    /**
     * @brief body creates a body action command for an arm
     * @return a body action command for an arm
     */
    static Arm body()
    {
      Arm arm;
      arm.type_ = MotionRequest::ArmMotion::BODY;
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
      arm.type_ = MotionRequest::ArmMotion::POINT;
      arm.target_ = target;
      return arm;
    }
    /**
     * @brief type returns the type of the command
     * @return the type of the command
     */
    MotionRequest::ArmMotion type() const
    {
      return type_;
    }

  private:
    /**
     * @brief Arm creates an undefined arm action command
     */
    Arm() = default;
    /// the requested arm motion type
    MotionRequest::ArmMotion type_;
    /// the (relative) point where the arm should point to
    Vector3f target_;
    friend class ActionCommand;
  };
  /**
   * @class Head contains the command for the head
   */
  class Head
  {
  public:
    /**
     * @brief body creates a body action command for the head
     * @return a body action command for the head
     */
    static Head body()
    {
      Head head;
      head.type_ = MotionRequest::HeadMotion::BODY;
      return head;
    }
    /**
     * @brief angles creates an angles action command for the head
     * @param yaw the desired yaw angle
     * @param pitch the desired pitch angle
     * @param yaw_velocity the maximal angular velocity of the yaw joint to reach the target (zero means maximal possible velocity)
     * @param pitch_velocity the maximal angular velocity of the pitch joint to reach the target (zero means maximal possible velocity)
     * @return an angles action command for the head
     */
    static Head angles(const float yaw = 0, const float pitch = 0, const float yaw_velocity = 0, const float pitch_velocity = 0)
    {
      Head head;
      head.type_ = MotionRequest::HeadMotion::ANGLES;
      head.yaw_ = yaw;
      head.pitch_ = pitch;
      head.yawVelocity_ = yaw_velocity;
      head.pitchVelocity_ = pitch_velocity;
      return head;
    }
    /**
     * @brief lookAt creates a lookAt action command for the head
     * @param target_position the target position in robot coordinates
     * @param yaw_velocity the maximal angular velocity of the yaw joint to reach the target (zero means maximal possible velocity)
     * @param pitch_velocity the maximal angular velocity of the pitch joint to reach the target (zero means maximal possible velocity)
     * @return a lookAt action command for the head
     */
    static Head lookAt(const Vector3f& target_position, const float yaw_velocity = 0, const float pitch_velocity = 0)
    {
      Head head;
      head.type_ = MotionRequest::HeadMotion::LOOK_AT;
      head.yawVelocity_ = yaw_velocity;
      head.pitchVelocity_ = pitch_velocity;
      head.target_position_ = target_position;
      return head;
    }
    /**
     * @brief type returns the type of the command
     * @return the type of the command
     */
    MotionRequest::HeadMotion type() const
    {
      return type_;
    }
    /**
     * @brief yaw returns the yaw of the command (only valid if type is ANGLES)
     * @return the yaw of the command
     */
    float yaw() const
    {
      return yaw_;
    }
    /**
     * @brief pitch returns the pitch of the command (only valid if type is ANGLES)
     * @return the pitch of the command
     */
    float pitch() const
    {
      return pitch_;
    }

  private:
    /**
     * @brief Head creates an undefined head action command
     */
    Head() = default;
    /// the requested head motion type
    MotionRequest::HeadMotion type_;
    /// the desired yaw angle
    float yaw_;
    /// the desired pitch angle
    float pitch_;
    /// the target to look at (in robot coordinates)
    Vector3f target_position_;
    /// the maximal angular velocity of the yaw joint to reach the target
    float yawVelocity_;
    /// the maximal angular velocity of the pitch joint to reach the target
    float pitchVelocity_;
    friend class ActionCommand;
  };
  /**
   * @class LED contains the command for an LED
   */
  class LED
  {
  public:
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
      led.r_ = r;
      led.g_ = g;
      led.b_ = b;
      return led;
    }
    /**
     * @brief off creates an off action command for an LED
     * @return an off action command for an LED
     */
    static LED off()
    {
      return colors(0, 0, 0);
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

  private:
    /**
     * @brief LED creates an undefined LED action command
     */
    LED() = default;
    /// the red intensity in [0,1]
    float r_;
    /// the green intensity in [0,1]
    float g_;
    /// the blue intensity in [0,1]
    float b_;
    friend class ActionCommand;
  };
  /**
   * @brief dead creates a dead action command
   * @return a dead action command
   */
  static ActionCommand dead()
  {
    return ActionCommand(Body::dead(), Arm::body(), Arm::body(), Head::body(), LED::colors(), LED::colors());
  }
  /**
   * @brief stand creates a stand action command
   * @return a stand action command
   */
  static ActionCommand stand()
  {
    return ActionCommand(Body::stand(), Arm::body(), Arm::body(), Head::angles(), LED::colors(), LED::colors());
  }
  /**
   * @brief walk creates a walk action command
   * @param target the (relative) pose where the robot should go
   * @param walkingMode specifies the mode of operation for the motionplanner like following path with fixed orientation
   * @param velocity Desired walking velocities, movement and rotation [percentage of max speed]
   * @param inWalkKickType the type of the in walk kick
   * @return a walk action command
   */
  static ActionCommand walk(const Pose& target, const WalkMode walkingMode = WalkMode::PATH, const Velocity& velocity = Velocity(),
                            const InWalkKickType inWalkKickType = InWalkKickType::NONE)
  {
    return ActionCommand(Body::walk(target, walkingMode, velocity, inWalkKickType), Arm::body(), Arm::body(), Head::angles(), LED::colors(), LED::colors());
  }
  /**
   * @brief walkVelocity creates an action command for walking according to the specified velocity, which contains direction and speed
   * @param velocity defines the translation direction and velocity as well as rotation velocity for walking [percentage of max speed]
   * @param inWalkKickType the type of the in walk kick, set to NONE if no kick is to be performed
   * return a walk action command for the body using the velocity walking mode
   */
  static ActionCommand walkVelocity(const Velocity& velocity, const InWalkKickType inWalkKickType = InWalkKickType::NONE)
  {
    // Use an empty pose for the target because it will be ignored in velocity mode
    return ActionCommand(Body::walk(Pose(), WalkMode::VELOCITY, velocity, inWalkKickType), Arm::body(), Arm::body(), Head::angles(), LED::colors(),
                         LED::colors());
  }
  /**
   * @brief kick creates a kick action command
   * @param ball_position the (relative) position where the kick should assume the ball to be
   * @param ball_target the (relative) position where the ball should end up
   * @param kickType the type of kick
   * @return a kick action command
   */
  static ActionCommand kick(const Vector2f& ball_position, const Vector2f& ball_target, const KickType kickType)
  {
    return ActionCommand(Body::kick(ball_position, ball_target, kickType), Arm::body(), Arm::body(), Head::body(), LED::colors(), LED::colors());
  }
  /**
   * @brief penalized creates a penalized action command
   * @return a penalized action command
   */
  static ActionCommand penalized()
  {
    return ActionCommand(Body::penalized(), Arm::body(), Arm::body(), Head::body(), LED::colors(), LED::colors());
  }
  /**
   * @brief keeper creates a keeper action command
   * @param keeper_type the type of the keeper motion
   * @return a keeper action command
   */
  static ActionCommand keeper(const MotionKeeper keeper_type)
  {
    return ActionCommand(Body::keeper(keeper_type), Arm::body(), Arm::body(), Head::body(), LED::colors(), LED::colors());
  }
  /**
   * @brief standUp creates a stand up action command
   * @return a stand up action command
   */
  static ActionCommand standUp()
  {
    return ActionCommand(Body::standUp(), Arm::body(), Arm::body(), Head::body(), LED::colors(), LED::colors());
  }
  /**
   * @brief hold creates a hold action command
   * @return a hold action command
   */
  static ActionCommand hold()
  {
    return ActionCommand(Body::hold(), Arm::body(), Arm::body(), Head::body(), LED::colors(), LED::colors());
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
  ActionCommand& combineLeftArm(const Arm& left_arm)
  {
    leftArm_ = left_arm;
    return *this;
  }
  /**
   * @brief combineRightArm replaces the right arm part of an action command
   * @param right_arm the new right arm part of the action command
   * @return reference to this
   */
  ActionCommand& combineRightArm(const Arm& right_arm)
  {
    rightArm_ = right_arm;
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
  ActionCommand& combineLeftLED(const LED& left_led)
  {
    leftLed_ = left_led;
    return *this;
  }
  /**
   * @brief combineRightLED replaces the right LED part of an action command
   * @param right_led the new right LED part of the action command
   * @return reference to this
   */
  ActionCommand& combineRightLED(const LED& right_led)
  {
    rightLed_ = right_led;
    return *this;
  }
  /**
   * @brief toMotionRequest converts the action command to a motion request
   * @param motion_request the motion request that is overwritten
   */
  void toMotionRequest(MotionRequest& motion_request) const
  {
    motion_request.bodyMotion = body_.type_;
    motion_request.walkData.target = body_.target_;
    motion_request.walkData.inWalkKickType = body_.inWalkKickType_;
    motion_request.walkData.mode = body_.walkingMode_;
    motion_request.walkData.velocity = body_.velocity_;
    motion_request.walkStopData.gracefully = false;
    motion_request.kickData.ballSource = body_.ballPosition_;
    motion_request.kickData.ballDestination = body_.ballTarget_;
    motion_request.kickData.kickType = body_.kickType_;
    motion_request.keeperData.keep = body_.keeperType_;
    if (!motion_request.usesArms())
    {
      motion_request.leftArmMotion = leftArm_.type_;
      motion_request.pointData.relativePoint = leftArm_.target_;
      motion_request.rightArmMotion = rightArm_.type_;
      // TODO: pointData for both arms?
    }
    else
    {
      motion_request.leftArmMotion = MotionRequest::ArmMotion::BODY;
      motion_request.rightArmMotion = MotionRequest::ArmMotion::BODY;
    }
    if (!motion_request.usesHead())
    {
      motion_request.headMotion = head_.type_;
      motion_request.headAngleData.headYaw = head_.yaw_;
      motion_request.headAngleData.headPitch = head_.pitch_;
      motion_request.headAngleData.maxHeadYawVelocity = head_.yawVelocity_;
      motion_request.headAngleData.maxHeadPitchVelocity = head_.pitchVelocity_;
      motion_request.headLookAtData.targetPosition = head_.target_position_;
      motion_request.headLookAtData.maxHeadYawVelocity = head_.yawVelocity_;
      motion_request.headLookAtData.maxHeadPitchVelocity = head_.pitchVelocity_;
    }
    else
    {
      motion_request.headMotion = MotionRequest::HeadMotion::BODY;
    }
  }
  /**
   * @brief toEyeLEDRequest converts the action command to an eye LED request
   * @param eyeLEDRequest the eye LED request that is overwritten
   */
  void toEyeLEDRequest(EyeLEDRequest& eyeLEDRequest) const
  {
    eyeLEDRequest.leftR = leftLed_.r_;
    eyeLEDRequest.leftG = leftLed_.g_;
    eyeLEDRequest.leftB = leftLed_.b_;
    eyeLEDRequest.rightR = rightLed_.r_;
    eyeLEDRequest.rightG = rightLed_.g_;
    eyeLEDRequest.rightB = rightLed_.b_;
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
    return leftLed_;
  }
  /**
   * @brief rightLED returns the right LED part of the command
   * @return the right LED part of the command
   */
  const LED& rightLED() const
  {
    return rightLed_;
  }

private:
  /**
   * @brief ActionCommand creates an action command from commands for every part
   * @param body the command for the body
   * @param left_arm the command for the left arm
   * @param right_arm the command for the right arm
   * @param head the command for the head
   * @param left_led the command for the left LED
   * @param right_led the command for the right LED
   */
  ActionCommand(const Body& body, const Arm& left_arm, const Arm& right_arm, const Head& head, const LED& left_led, const LED& right_led)
    : body_(body)
    , leftArm_(left_arm)
    , rightArm_(right_arm)
    , head_(head)
    , leftLed_(left_led)
    , rightLed_(right_led)
  {
  }
  /// the command for the body
  Body body_;
  /// the command for the left arm
  Arm leftArm_;
  /// the command for the right arm
  Arm rightArm_;
  /// the command for the head
  Head head_;
  /// the command for the left LED
  LED leftLed_;
  /// the command for the right LED
  LED rightLed_;
};
