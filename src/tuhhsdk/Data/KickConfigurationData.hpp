#pragma once

#include "Framework/DataType.hpp"
#include "Tools/Math/Angle.hpp"
#include "Tools/Math/Pose.hpp"

#include <array>

enum class InWalkKickType {
  NONE,
  FORWARD,
  TURN,
  MAX
};

enum class KickFoot {
  NONE,
  LEFT,
  RIGHT
};

struct InWalkKick : public Uni::To, public Uni::From
{
  InWalkKick(const bool requiresPrestep = false, const Pose& preStep = Pose(), const Pose& kickStep = Pose(),
             const float distanceToBallX = 0.2f, const float distanceToBallY = 0.05f, const float kickDirectionAngle = 0.f)
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

  virtual void toValue(Uni::Value& value) const
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["requiresPrestep"] << requiresPrestep;
    value["kickStep"] << kickStep;
    value["preStep"] << preStep;
    value["distanceToBallX"] << distanceToBallX;
    value["distanceToBallY"] << distanceToBallY;
    value["kickDirectionAngle"] << kickDirectionAngle;
  }

  virtual void fromValue(const Uni::Value& value)
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
  /// the array of availalbe in walk kicks
  std::array<InWalkKick, static_cast<unsigned int>(InWalkKickType::MAX)> inWalkKicks;

  void reset() {}

  virtual void toValue(Uni::Value& value) const
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["inWalkKicks"] << inWalkKicks;
  }

  virtual void fromValue(const Uni::Value& value)
  {
    value["inWalkKicks"] >> inWalkKicks;
  }
};
