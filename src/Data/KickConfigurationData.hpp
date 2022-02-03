#pragma once

#include "Framework/DataType.hpp"
#include "Hardware/Clock.hpp"
#include "Tools/Math/Angle.hpp"
#include "Tools/Math/Pose.hpp"
#include "Tools/Storage/EnumArray.hpp"
#include <algorithm>
#include <array>
#include <utility>

enum class KickType
{
  NONE,
  FORWARD,
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
  explicit KickConfiguration(Vector2f distanceToBall = Vector2f::Zero(),
                             Clock::duration waitBeforeStartDuration = Clock::duration{},
                             Clock::duration weightShiftDuration = Clock::duration{},
                             Clock::duration liftFootDuration = Clock::duration{},
                             Clock::duration kickAccelerationDuration = Clock::duration{},
                             Clock::duration kickConstantDuration = Clock::duration{},
                             Clock::duration kickDecelerationDuration = Clock::duration{},
                             Clock::duration retractFootDuration = Clock::duration{},
                             Clock::duration extendFootAndCenterTorsoDuration = Clock::duration{},
                             Clock::duration waitBeforeExitDuration = Clock::duration{},
                             Vector3f weightShiftCom = Vector3f::Zero(),
                             Vector3f liftFootPosition = Vector3f::Zero(),
                             Vector3f kickAccelerationPosition = Vector3f::Zero(),
                             Vector3f kickConstantPosition = Vector3f::Zero(),
                             Vector3f kickDecelerationPosition = Vector3f::Zero(),
                             Vector3f retractFootPosition = Vector3f::Zero(),
                             const float yawLeft2right = 0.f, const float shoulderRoll = 0.f,
                             const float shoulderPitchAdjustment = 0.f, const float ankleRoll = 0.f,
                             const float anklePitch = 0.f)
    : distanceToBall{std::move(distanceToBall)}
    , waitBeforeStartDuration{waitBeforeStartDuration}
    , weightShiftDuration{weightShiftDuration}
    , liftFootDuration{liftFootDuration}
    , kickAccelerationDuration{kickAccelerationDuration}
    , kickConstantDuration{kickConstantDuration}
    , kickDecelerationDuration{kickDecelerationDuration}
    , retractFootDuration{retractFootDuration}
    , extendFootAndCenterTorsoDuration{extendFootAndCenterTorsoDuration}
    , waitBeforeExitDuration{waitBeforeExitDuration}
    , weightShiftCom{std::move(weightShiftCom)}
    , liftFootPosition{std::move(liftFootPosition)}
    , kickAccelerationPosition{std::move(kickAccelerationPosition)}
    , kickConstantPosition{std::move(kickConstantPosition)}
    , kickDecelerationPosition{std::move(kickDecelerationPosition)}
    , retractFootPosition{std::move(retractFootPosition)}
    , yawLeft2right{yawLeft2right}
    , shoulderRoll{shoulderRoll}
    , shoulderPitchAdjustment{shoulderPitchAdjustment}
    , ankleRoll{ankleRoll}
    , anklePitch{anklePitch}
  {
  }

  /// the distance to the ball (for the kick pose)
  Vector2f distanceToBall;
  Clock::duration waitBeforeStartDuration;
  Clock::duration weightShiftDuration;
  Clock::duration liftFootDuration;
  Clock::duration kickAccelerationDuration;
  Clock::duration kickConstantDuration;
  Clock::duration kickDecelerationDuration;
  Clock::duration retractFootDuration;
  Clock::duration extendFootAndCenterTorsoDuration;
  Clock::duration waitBeforeExitDuration;
  /// position of CoM after weight shift
  Vector3f weightShiftCom;
  /// position of kick foot after lifting it
  Vector3f liftFootPosition;
  /// position of kick foot after swinging it
  Vector3f kickAccelerationPosition;
  /// position of kick foot exactly at ball
  Vector3f kickConstantPosition;
  /// position of kick foot after kicking the ball
  Vector3f kickDecelerationPosition;
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
    value["kickAccelerationDuration"] << kickAccelerationDuration;
    value["kickConstantDuration"] << kickConstantDuration;
    value["kickDecelerationDuration"] << kickDecelerationDuration;
    value["retractFootDuration"] << retractFootDuration;
    value["extendFootAndCenterTorsoDuration"] << extendFootAndCenterTorsoDuration;
    value["waitBeforeExitDuration"] << waitBeforeExitDuration;
    value["weightShiftCom"] << weightShiftCom;
    value["liftFootPosition"] << liftFootPosition;
    value["kickAccelerationPosition"] << kickAccelerationPosition;
    value["kickConstantPosition"] << kickConstantPosition;
    value["kickDecelerationPosition"] << kickDecelerationPosition;
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
    value["kickAccelerationDuration"] >> kickAccelerationDuration;
    value["kickConstantDuration"] >> kickConstantDuration;
    value["kickDecelerationDuration"] >> kickDecelerationDuration;
    value["retractFootDuration"] >> retractFootDuration;
    value["extendFootAndCenterTorsoDuration"] >> extendFootAndCenterTorsoDuration;
    value["waitBeforeExitDuration"] >> waitBeforeExitDuration;
    value["weightShiftCom"] >> weightShiftCom;
    value["liftFootPosition"] >> liftFootPosition;
    value["kickAccelerationPosition"] >> kickAccelerationPosition;
    value["kickConstantPosition"] >> kickConstantPosition;
    value["kickDecelerationPosition"] >> kickDecelerationPosition;
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
  explicit InWalkKick(const bool requiresPrestep = false, Pose preStep = Pose(),
                      Pose kickStep = Pose(), const float distanceToBallX = 0.2f,
                      const float distanceToBallY = 0.05f, const float kickDirectionAngle = 0.f)
    : requiresPrestep(requiresPrestep)
    , kickStep(std::move(kickStep))
    , preStep(std::move(preStep))
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
  DataTypeName name__{"KickConfigurationData"};
  /// the array of availalbe kicks
  EnumArray<KickConfiguration, KickType, static_cast<std::size_t>(KickType::MAX)> kicks;
  /// the array of availalbe in walk kicks
  EnumArray<InWalkKick, InWalkKickType, static_cast<std::size_t>(InWalkKickType::MAX)> inWalkKicks;

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
