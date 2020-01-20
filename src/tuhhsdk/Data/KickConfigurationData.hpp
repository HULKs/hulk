#pragma once

#include "Framework/DataType.hpp"
#include "Tools/Math/Angle.hpp"
#include "Tools/Math/Pose.hpp"

#include <array>

enum class KickType
{
  NONE,
  FORWARD,
  SIDE,
  MAX
};

enum class InWalkKickType
{
  NONE,
  FORWARD,
  TURN,
  MAX
};

enum class KickFoot
{
  NONE,
  LEFT,
  RIGHT
};

struct KickConfiguration : public Uni::To, public Uni::From
{
  explicit KickConfiguration(
      const Vector2f distanceToBall = Vector2f::Zero(),
      const unsigned int waitBeforeStartDuration = 0, const unsigned int weightShiftDuration = 0,
      const unsigned int liftFootDuration = 0, const unsigned int swingFootDuration = 0,
      const unsigned int kickBallDuration = 0, const unsigned int pauseDuration = 0,
      const unsigned int retractFootDuration = 0,
      const unsigned int extendFootAndCenterTorsoDuration = 0,
      const unsigned int waitBeforeExitDuration = 0,
      const Vector3f weightShiftCom = Vector3f::Zero(),
      const Vector3f liftFootPosition = Vector3f::Zero(),
      const Vector3f swingFootPosition = Vector3f::Zero(),
      const Vector3f kickBallPosition = Vector3f::Zero(),
      const Vector3f retractFootPosition = Vector3f::Zero(), const float yawLeft2right = 0.f,
      const float shoulderRoll = 0.f, const float shoulderPitchAdjustment = 0.f,
      const float ankleRoll = 0.f, const float anklePitch = 0.f)
    : distanceToBall(distanceToBall)
    , waitBeforeStartDuration(waitBeforeStartDuration)
    , weightShiftDuration(weightShiftDuration)
    , liftFootDuration(liftFootDuration)
    , swingFootDuration(swingFootDuration)
    , kickBallDuration(kickBallDuration)
    , pauseDuration(pauseDuration)
    , retractFootDuration(retractFootDuration)
    , extendFootAndCenterTorsoDuration(extendFootAndCenterTorsoDuration)
    , waitBeforeExitDuration(waitBeforeExitDuration)
    , weightShiftCom(weightShiftCom)
    , liftFootPosition(liftFootPosition)
    , swingFootPosition(swingFootPosition)
    , kickBallPosition(kickBallPosition)
    , retractFootPosition(retractFootPosition)
    , yawLeft2right(yawLeft2right)
    , shoulderRoll(shoulderRoll)
    , shoulderPitchAdjustment(shoulderPitchAdjustment)
    , ankleRoll(ankleRoll)
    , anklePitch(anklePitch)
  {
  }

  /// the distance to the ball (for the kick pose)
  Vector2f distanceToBall;
  unsigned int waitBeforeStartDuration;
  unsigned int weightShiftDuration;
  unsigned int liftFootDuration;
  unsigned int swingFootDuration;
  unsigned int kickBallDuration;
  unsigned int pauseDuration;
  unsigned int retractFootDuration;
  unsigned int extendFootAndCenterTorsoDuration;
  unsigned int waitBeforeExitDuration;
  /// position of CoM after weight shift
  Vector3f weightShiftCom;
  /// position of kick foot after lifting it
  Vector3f liftFootPosition;
  /// position of kick foot after swinging it
  Vector3f swingFootPosition;
  /// position of kick foot after kicking the ball
  Vector3f kickBallPosition;
  /// position of kick foot after retracting it it
  Vector3f retractFootPosition;
  /// yawLeft2Right is the only joint angle that affects the yaw between the feet
  float yawLeft2right;
  /// shoulder roll prevents collision of arms with body
  float shoulderRoll;
  /// shoulderPitchAdjustement is added to shoulder pitch for momentum compnsation
  float shoulderPitchAdjustment;
  float ankleRoll;
  float anklePitch;

  void toValue(Uni::Value& value) const override
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["distanceToBall"] << distanceToBall;
    value["waitBeforeExitDuration"] << waitBeforeStartDuration;
    value["weightShiftDuration"] << weightShiftDuration;
    value["liftFootDuration"] << liftFootDuration;
    value["swingFootDuration"] << swingFootDuration;
    value["kickBallDuration"] << kickBallDuration;
    value["pauseDuration"] << pauseDuration;
    value["retractFootDuration"] << retractFootDuration;
    value["extendFootAndCenterTorsoDuration"] << extendFootAndCenterTorsoDuration;
    value["waitBeforeExitDuration"] << waitBeforeExitDuration;
    value["weightShiftCom"] << weightShiftCom;
    value["liftFootPosition"] << liftFootPosition;
    value["swingFootPosition"] << swingFootPosition;
    value["kickBallPosition"] << kickBallPosition;
    value["retractFootPosition"] << retractFootPosition;
    value["yawLeft2right"] << yawLeft2right;
    value["shoulderRoll"] << shoulderRoll;
    value["shoulderPitchAdjustment"] << shoulderPitchAdjustment;
    value["ankleRoll"] << ankleRoll;
    value["anklePitch"] << anklePitch;
  }

  void fromValue(const Uni::Value& value) override
  {
    value["distanceToBall"] >> distanceToBall;
    value["waitBeforeStartDuration"] >> waitBeforeStartDuration;
    value["weightShiftDuration"] >> weightShiftDuration;
    value["liftFootDuration"] >> liftFootDuration;
    value["swingFootDuration"] >> swingFootDuration;
    value["kickBallDuration"] >> kickBallDuration;
    value["pauseDuration"] >> pauseDuration;
    value["retractFootDuration"] >> retractFootDuration;
    value["extendFootAndCenterTorsoDuration"] >> extendFootAndCenterTorsoDuration;
    value["waitBeforeExitDuration"] >> waitBeforeExitDuration;
    value["weightShiftCom"] >> weightShiftCom;
    value["liftFootPosition"] >> liftFootPosition;
    value["swingFootPosition"] >> swingFootPosition;
    value["kickBallPosition"] >> kickBallPosition;
    value["retractFootPosition"] >> retractFootPosition;
    value["yawLeft2right"] >> yawLeft2right;
    value["shoulderRoll"] >> shoulderRoll;
    value["shoulderPitchAdjustment"] >> shoulderPitchAdjustment;
    value["ankleRoll"] >> ankleRoll;
    value["anklePitch"] >> anklePitch;
  }
};

struct InWalkKick : public Uni::To, public Uni::From
{
  explicit InWalkKick(const bool requiresPrestep = false, const Pose& preStep = Pose(),
                      const Pose& kickStep = Pose(), const float distanceToBallX = 0.2f,
                      const float distanceToBallY = 0.05f, const float kickDirectionAngle = 0.f)
    : requiresPrestep(requiresPrestep)
    , kickStep(kickStep)
    , preStep(preStep)
    , distanceToBallX(distanceToBallX)
    , distanceToBallY(distanceToBallY)
    , kickDirectionAngle(kickDirectionAngle)
  {
  }
  /// true if this step requires a certain pre step of the non kicking foot
  bool requiresPrestep;
  /// the kick step size if the kicking foot was the left one (mirrored if necessary)
  Pose kickStep;
  /// the pre step size if the non-kicking foot was the right one
  Pose preStep;
  /// the distance from the ball in x direction (for the kickPose)
  float distanceToBallX;
  /// the distance from the ball in y direction (for the kickPose)
  float distanceToBallY;
  /// the direction the ball will go when this kick is performed with the left foot
  float kickDirectionAngle;

  void toValue(Uni::Value& value) const override
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["requiresPrestep"] << requiresPrestep;
    value["kickStep"] << kickStep;
    value["preStep"] << preStep;
    value["distanceToBallX"] << distanceToBallX;
    value["distanceToBallY"] << distanceToBallY;
    value["kickDirectionAngle"] << kickDirectionAngle;
  }

  void fromValue(const Uni::Value& value) override
  {
    value["requiresPrestep"] >> requiresPrestep;
    value["kickStep"] >> kickStep;
    value["preStep"] >> preStep;
    value["distanceToBallX"] >> distanceToBallX;
    value["distanceToBallY"] >> distanceToBallY;
    value["kickDirectionAngle"] >> kickDirectionAngle;
  }
};


/**
 * @brief KickConfigurationData contains meta data in configuration to perform kicks. These are on
 * the one hand information for brain (e.g. kick pose) and on the other hand information to actually
 * perform the kick (e.g. kick steps for in walk kicks)
 */
class KickConfigurationData : public DataType<KickConfigurationData>
{
public:
  /// the name of this DataType
  DataTypeName name = "KickConfigurationData";
  /// the array of availalbe kicks
  std::array<KickConfiguration, static_cast<unsigned int>(KickType::MAX)> kicks;
  /// the array of availalbe in walk kicks
  std::array<InWalkKick, static_cast<unsigned int>(InWalkKickType::MAX)> inWalkKicks;

  void reset() override {}

  void toValue(Uni::Value& value) const override
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["inWalkKicks"] << inWalkKicks;
    value["kicks"] << kicks;
  }

  void fromValue(const Uni::Value& value) override
  {
    value["inWalkKicks"] >> inWalkKicks;
    value["kicks"] >> kicks;
  }
};
