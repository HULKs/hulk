#include "Modules/Poses.h"
#include "Tools/Kinematics/Com.h"
#include "Tools/Kinematics/ForwardKinematics.h"
#include "Tools/Kinematics/InverseKinematics.h"

#include "KickPhase.hpp"


KickPhaseHelper::KickPhaseHelper(const ModuleBase& module)
  : previousAngles_(Poses::getPose(Poses::READY))
  , comSingleSupportLeftKicking_(module, "comSingleSupportLeftKicking", [] {})
  , comSingleSupportRightKicking_(module, "comSingleSupportRightKicking", [] {})
  , comExtendAndCenterLeftKicking_(module, "comExtendAndCenterLeftKicking", [] {})
  , comExtendAndCenterRightKicking_(module, "comExtendAndCenterRightKicking", [] {})
  , liftPositionModifier_(module, "liftPositionModifier", [] {})
  , swingPositionModifier_(module, "swingPositionModifier", [] {})
  , liftHeight_(module, "liftHeight", [] {})
  , swingHeight_(module, "swingHeight", [] {})
  , liftMin_(module, "liftMin", [] {})
  , liftMax_(module, "liftMax", [] {})
  , swingMin_(module, "swingMin", [] {})
  , swingMax_(module, "swingMax", [] {})
{
}

void KickPhaseHelper::resetStraightKick(const bool leftKicking, const Vector2f ballSource, const Vector2f ballDestination,
                                        const std::vector<float> anglesAtKickRequest)
{
  std::vector<float> lLegAngles, rLegAngles;
  separateAngles(lLegAngles, rLegAngles, anglesAtKickRequest);
  int sign;
  KinematicMatrix torso2supportFootAtKickRequest;
  /// some things are different depending on which foot is kicking
  if (leftKicking)
  {
    sign = 1;
    torso2supportFootAtKickRequest = ForwardKinematics::getRFoot(rLegAngles).invert();
    comSingleSupport_ = comSingleSupportLeftKicking_();
    comExtendAndCenter_ = comExtendAndCenterLeftKicking_();
  }
  else
  {
    sign = -1;
    torso2supportFootAtKickRequest = ForwardKinematics::getLFoot(lLegAngles).invert();
    comSingleSupport_ = comSingleSupportRightKicking_();
    comExtendAndCenter_ = comExtendAndCenterRightKicking_();
  }

  const Vector2f kickDirectionNormalized = (ballDestination - ballSource).normalized();
  const Vector2f torso2supportFootVector2 = Vector2f(torso2supportFootAtKickRequest.posV.x(), sign * torso2supportFootAtKickRequest.posV.y());
  const Vector2f ballSourceInMM = Vector2f(ballSource.x(), sign * ballSource.y()) * 1000;
  const Vector2f tip2ankle = Vector2f(120, 0);

  const Vector2f lp = torso2supportFootVector2 + ballSourceInMM - tip2ankle - kickDirectionNormalized * liftPositionModifier_();
  liftPosition_ = Vector3f(lp.x(), lp.y(), liftHeight_());
  const Vector2f sp = torso2supportFootVector2 + ballSourceInMM - tip2ankle + kickDirectionNormalized * swingPositionModifier_();
  swingPosition_ = Vector3f(sp.x(), sp.y(), swingHeight_());

  /// limit positions to avoid collisions
  for (unsigned int it = 0; it < 2; it++)
  {
    liftPosition_[it] = clamp(liftPosition_[it], liftMin_()[it], liftMax_()[it]);
    swingPosition_[it] = clamp(swingPosition_[it], swingMin_()[it], swingMax_()[it]);
  }
}

template <typename T>
T KickPhaseHelper::clamp(const T value, const T min, const T max)
{
  return std::min(max, std::max(min, value));
}

void KickPhaseHelper::separateAngles(std::vector<float>& left, std::vector<float>& right, const std::vector<float>& body)
{
  left.resize(JOINTS_L_LEG::L_LEG_MAX);
  right.resize(JOINTS_R_LEG::R_LEG_MAX);
  for (unsigned int i = 0; i < JOINTS_L_LEG::L_LEG_MAX; i++)
  {
    left[i] = body[JOINTS::L_HIP_YAW_PITCH + i];
  }
  for (unsigned int i = 0; i < JOINTS_R_LEG::R_LEG_MAX; i++)
  {
    right[i] = body[JOINTS::R_HIP_YAW_PITCH + i];
  }
}

void KickPhaseHelper::combineAngles(std::vector<float>& result, const std::vector<float>& body, const std::vector<float>& left, const std::vector<float>& right)
{
  result = body;
  for (unsigned int i = 0; i < JOINTS_L_LEG::L_LEG_MAX; i++)
  {
    result[JOINTS::L_HIP_YAW_PITCH + i] = left[i];
  }
  for (unsigned int i = 0; i < JOINTS_R_LEG::R_LEG_MAX; i++)
  {
    result[JOINTS::R_HIP_YAW_PITCH + i] = right[i];
  }
}

Vector3f KickPhaseHelper::getLiftPosition()
{
  return liftPosition_;
}

Vector3f KickPhaseHelper::getSwingPosition()
{
  return swingPosition_;
}

void KickPhaseHelper::setPreviousAngles(const std::vector<float>& previousAngles)
{
  previousAngles_ = previousAngles;
}

void KickPhaseHelper::getPreviousAngles(std::vector<float>& previousAngles)
{
  previousAngles = previousAngles_;
}

std::vector<float> KickPhaseHelper::getComSingleSupport()
{
  return comSingleSupport_;
}

std::vector<float> KickPhaseHelper::getComExtendAndCenter()
{
  return comExtendAndCenter_;
}

/// kick phase
KickPhase::KickPhase(const ModuleBase& module, KickPhaseHelper& kickPhaseHelper, const unsigned int duration)
  : kickPhaseHelper_(kickPhaseHelper)
  , duration_(duration)
  , torsoRoll_(module, "torsoRoll", [] {})
  , liftAnklePitch_(module, "liftAnklePitch", [] {})
  , swingAnklePitch_(module, "swingAnklePitch", [] {})
{
}

void KickPhase::computeLegAngles(std::vector<float>& bodyAngles, const KinematicMatrix com2rightDesired, const KinematicMatrix left2rightDesired,
                                 const float torsoRoll)
{
  std::vector<float> previousAngles, lLegAngles, rLegAngles;
  kickPhaseHelper_.getPreviousAngles(previousAngles);
  KinematicMatrix com2torso = Com::getCom(previousAngles);
  kickPhaseHelper_.separateAngles(lLegAngles, rLegAngles, previousAngles);
  KinematicMatrix right2torso = ForwardKinematics::getRFoot(rLegAngles);
  right2torso.rotM = AngleAxisf(torsoRoll * TO_RAD, Vector3f::UnitX());
  const KinematicMatrix right2com = com2torso.invert() * right2torso;
  const KinematicMatrix left2com = right2com * left2rightDesired;

  /// iteratively find angles that allow the com to move to a desired position
  for (int i = 0; i < 5; i++)
  {
    /// compute leg angles
    rLegAngles = InverseKinematics::getRLegAngles(com2torso * right2com);
    lLegAngles = InverseKinematics::getFixedLLegAngles(com2torso * left2com, rLegAngles[0]);

    /// compute new com2right
    kickPhaseHelper_.combineAngles(bodyAngles, Poses::getPose(Poses::READY), lLegAngles, rLegAngles);
    com2torso = KinematicMatrix(Com::getCom(bodyAngles));
    right2torso = ForwardKinematics::getRFoot(rLegAngles);
    KinematicMatrix com2right = right2torso.invert() * com2torso;

    /// shift com2torso
    Vector3f error = com2right.posV - com2rightDesired.posV;
    com2torso.posV.x() += error.x();
    com2torso.posV.y() += error.y();
  }
  kickPhaseHelper_.combineAngles(bodyAngles, Poses::getPose(Poses::READY), lLegAngles, rLegAngles);
  kickPhaseHelper_.setPreviousAngles(bodyAngles);
}

/// to ready phase
ToReady::ToReady(const ModuleBase& module, KickPhaseHelper& kickPhaseHelper, const unsigned int duration)
  : KickPhase(module, kickPhaseHelper, duration)
{
}

void ToReady::reset(const std::vector<float>& previousAngles)
{
  kickPhaseHelper_.setPreviousAngles(previousAngles);
  interpolator_.reset(previousAngles, Poses::getPose(Poses::READY), duration_);
}

void ToReady::getAngles(std::vector<float>& bodyAngles, const unsigned int dt)
{
  bodyAngles = interpolator_.step(dt);
}

bool ToReady::finished()
{
  return interpolator_.finished();
}

/// balance phase
Balance::Balance(const ModuleBase& module, KickPhaseHelper& kickPhaseHelper, const unsigned int duration)
  : KickPhase(module, kickPhaseHelper, duration)
{
}

void Balance::reset(const std::vector<float>& previousAngles)
{
  std::vector<float> lLegAngles, rLegAngles;
  kickPhaseHelper_.separateAngles(lLegAngles, rLegAngles, previousAngles);
  const KinematicMatrix left2torso = ForwardKinematics::getLFoot(lLegAngles);
  const KinematicMatrix right2torso = ForwardKinematics::getRFoot(rLegAngles);
  KinematicMatrix com2torso = KinematicMatrix(Com::getCom(previousAngles));

  /// balance nao by shifting the com to the support foot
  const KinematicMatrix com2rightInitial = right2torso.invert() * com2torso;
  std::vector<float> comInitial = {com2rightInitial.posV.x(), com2rightInitial.posV.y(), com2rightInitial.posV.z()};
  KinematicMatrix left2right = right2torso.invert() * left2torso;
  left2right.posV.z() = 0;
  const std::vector<float> comBalance = kickPhaseHelper_.getComSingleSupport();
  const std::vector<float> readyTorsoAngle = {0};
  const std::vector<float> balanceTorsoAngle = {torsoRoll_()};

  comInterpolator_.reset(comInitial, comBalance, duration_);
  torsoAngleInterpolator_.reset(readyTorsoAngle, balanceTorsoAngle, duration_);
  progress_ = 0.f;
  left2right_ = left2right;
}

void Balance::getAngles(std::vector<float>& bodyAngles, const unsigned int dt)
{
  progress_ += dt / (float)duration_;
  const float step = 2.2f - 2 * progress_;
  const std::vector<float> comPosition = comInterpolator_.step(dt * step);
  const KinematicMatrix com2right = KinematicMatrix(Vector3f(comPosition[0], comPosition[1], comPosition[2]));
  const std::vector<float> torsoAngle = torsoAngleInterpolator_.step(dt);
  KickPhase::computeLegAngles(bodyAngles, com2right, left2right_, torsoAngle[0]);
}

bool Balance::finished()
{
  return comInterpolator_.finished() && torsoAngleInterpolator_.finished();
}

Lift::Lift(const ModuleBase& module, KickPhaseHelper& kickPhaseHelper, const unsigned int duration)
  : KickPhase(module, kickPhaseHelper, duration)
{
}

void Lift::reset(const std::vector<float>& previousAngles)
{
  /// foot position at end of balance phase
  std::vector<float> lLegAngles, rLegAngles;
  kickPhaseHelper_.separateAngles(lLegAngles, rLegAngles, previousAngles);
  const KinematicMatrix left2torso = ForwardKinematics::getLFoot(lLegAngles);
  const KinematicMatrix right2torso = ForwardKinematics::getRFoot(rLegAngles);
  const KinematicMatrix left2right = right2torso.invert() * left2torso;
  const std::vector<float> balancePosition = {left2right.posV.x(), left2right.posV.y(), left2right.posV.z()};

  const Vector3f lp = kickPhaseHelper_.getLiftPosition();
  const std::vector<float> liftPosition = {lp.x(), lp.y(), lp.z()};
  const std::vector<float> comLift = kickPhaseHelper_.getComSingleSupport();
  const std::vector<float> balanceAnklePitch = {0};
  const std::vector<float> liftAnklePitch = {liftAnklePitch_() * TO_RAD};

  liftInterpolator_.reset(balancePosition, liftPosition, duration_);
  anklePitchInterpolator_.reset(balanceAnklePitch, liftAnklePitch, duration_);
  com2right_ = KinematicMatrix(Vector3f(comLift[0], comLift[1], comLift[2]));
}

void Lift::getAngles(std::vector<float>& bodyAngles, const unsigned int dt)
{
  const std::vector<float> liftPosition = liftInterpolator_.step(dt);
  const float anklePitch = anklePitchInterpolator_.step(dt)[0];
  const KinematicMatrix left2right = KinematicMatrix(AngleAxisf(anklePitch, Vector3f::UnitY()), Vector3f(liftPosition[0], liftPosition[1], liftPosition[2]));
  KickPhase::computeLegAngles(bodyAngles, com2right_, left2right, torsoRoll_());
}

bool Lift::finished()
{
  return liftInterpolator_.finished();
}

/// lift and kick phase
Swing::Swing(const ModuleBase& module, KickPhaseHelper& kickPhaseHelper, unsigned int duration)
  : KickPhase(module, kickPhaseHelper, duration)
  , canonicalSystemFinalValue_(module, "canonicalSystemFinalValue", [] {})
  , weightings_(module, "weightings", [] {})
  , dynamicMovementPrimitive_(canonicalSystemFinalValue_(), weightings_())
{
}

void Swing::reset(const std::vector<float>& previousAngles)
{
  /// foot position at end of lift phase
  std::vector<float> lLegAngles, rLegAngles;
  kickPhaseHelper_.separateAngles(lLegAngles, rLegAngles, previousAngles);
  const KinematicMatrix left2torso = ForwardKinematics::getLFoot(lLegAngles);
  const KinematicMatrix right2torso = ForwardKinematics::getRFoot(rLegAngles);
  const KinematicMatrix left2right = right2torso.invert() * left2torso;
  const Vector3f liftPosition = Vector3f(left2right.posV.x(), left2right.posV.y(), left2right.posV.z());

  const Vector3f swingPosition = kickPhaseHelper_.getSwingPosition();
  const std::vector<float> comSwing = kickPhaseHelper_.getComSingleSupport();

  const std::vector<float> liftAnklePitch = {liftAnklePitch_()};
  const std::vector<float> swingAnklePitch = {swingAnklePitch_() * TO_RAD};

  dynamicMovementPrimitive_.reset(liftPosition, swingPosition, duration_);
  anklePitchInterpolator_.reset(liftAnklePitch, swingAnklePitch, duration_);
  com2right_ = KinematicMatrix(Vector3f(comSwing[0], comSwing[1], comSwing[2]));
}

void Swing::getAngles(std::vector<float>& bodyAngles, unsigned int dt)
{
  const Vector3f swingPosition = dynamicMovementPrimitive_.step(dt);
  const KinematicMatrix left2right = KinematicMatrix(AngleAxisf(liftAnklePitch_() * TO_RAD, Vector3f::UnitY()), swingPosition);
  KickPhase::computeLegAngles(bodyAngles, com2right_, left2right, torsoRoll_());
}

bool Swing::finished()
{
  return dynamicMovementPrimitive_.finished();
}

/// retract phase
Retract::Retract(const ModuleBase& module, KickPhaseHelper& kickPhaseHelper, const unsigned int duration)
  : KickPhase(module, kickPhaseHelper, duration)
  , retractPosition_(module, "retractPosition", [] {})
{
}

void Retract::reset(const std::vector<float>& previousAngles)
{
  /// foot position at end of swing phase
  std::vector<float> lLegAngles, rLegAngles;
  kickPhaseHelper_.separateAngles(lLegAngles, rLegAngles, previousAngles);
  const KinematicMatrix left2torso = ForwardKinematics::getLFoot(lLegAngles);
  const KinematicMatrix right2torso = ForwardKinematics::getRFoot(rLegAngles);
  const KinematicMatrix left2right = right2torso.invert() * left2torso;
  const std::vector<float> swingPosition = {left2right.posV.x(), left2right.posV.y(), left2right.posV.z()};

  const std::vector<float> retractPosition = {retractPosition_()[0], retractPosition_()[1], retractPosition_()[2]};
  const std::vector<float> comRetract = kickPhaseHelper_.getComSingleSupport();
  const std::vector<float> swingAnklePitch = {swingAnklePitch_() * TO_RAD};
  const std::vector<float> retractAnklePitch = {0};

  retractInterpolator_.reset(swingPosition, retractPosition, duration_);
  anklePitchInterpolator_.reset(swingAnklePitch, retractAnklePitch, duration_);
  com2right_ = KinematicMatrix(Vector3f(comRetract[0], comRetract[1], comRetract[2]));
}

void Retract::getAngles(std::vector<float>& bodyAngles, const unsigned int dt)
{
  const std::vector<float> retractPosition = retractInterpolator_.step(dt);
  const float anklePitch = anklePitchInterpolator_.step(dt)[0];
  const KinematicMatrix left2right =
      KinematicMatrix(AngleAxisf(anklePitch, Vector3f::UnitY()), Vector3f(retractPosition[0], retractPosition[1], retractPosition[2]));
  KickPhase::computeLegAngles(bodyAngles, com2right_, left2right, torsoRoll_());
}

bool Retract::finished()
{
  return retractInterpolator_.finished();
}

/// extend and center phase
ExtendAndCenter::ExtendAndCenter(const ModuleBase& module, KickPhaseHelper& kickPhaseHelper, const unsigned int duration)
  : KickPhase(module, kickPhaseHelper, duration)
  , retractPosition_(module, "retractPosition", [] {})
  , extendPosition_(module, "extendPosition", [] {})
{
}

void ExtendAndCenter::reset()
{
  const std::vector<float> comSingleSupport = kickPhaseHelper_.getComSingleSupport();
  const std::vector<float> comExtendAndCenter = kickPhaseHelper_.getComExtendAndCenter();
  const std::vector<float> retractPosition = {retractPosition_()[0], retractPosition_()[1], retractPosition_()[2]};
  const std::vector<float> extendPosition = {extendPosition_()[0], extendPosition_()[1], extendPosition_()[2]};
  const std::vector<float> retractTorsoAngle = {torsoRoll_()};
  const std::vector<float> extendTorsoAngle = {0};

  extendInterpolator_.reset(retractPosition, extendPosition, duration_);
  centerInterpolator_.reset(comSingleSupport, comExtendAndCenter, duration_);
  torsoAngleInterpolator_.reset(retractTorsoAngle, extendTorsoAngle, duration_);
}

void ExtendAndCenter::getAngles(std::vector<float>& bodyAngles, const unsigned int dt)
{
  const std::vector<float> comPosition = centerInterpolator_.step(dt);
  const KinematicMatrix com2right = KinematicMatrix(Vector3f(comPosition[0], comPosition[1], comPosition[2]));
  const std::vector<float> extendPosition = extendInterpolator_.step(dt);
  const KinematicMatrix left2right = KinematicMatrix(Vector3f(extendPosition[0], extendPosition[1], extendPosition[2]));
  const std::vector<float> torsoAngle = torsoAngleInterpolator_.step(dt);
  KickPhase::computeLegAngles(bodyAngles, com2right, left2right, torsoAngle[0]);
}

bool ExtendAndCenter::finished()
{
  return extendInterpolator_.finished() && centerInterpolator_.finished() && torsoAngleInterpolator_.finished();
}

/// wait phase
Wait::Wait(const ModuleBase& module, KickPhaseHelper& kickPhaseHelper, const unsigned int duration)
  : KickPhase(module, kickPhaseHelper, duration)
{
}

void Wait::reset(const std::vector<float>& previousAngles)
{
  const std::vector<float> waitAngles = Poses::getPose(Poses::READY);
  interpolator_.reset(previousAngles, waitAngles, duration_);
}

void Wait::getAngles(std::vector<float>& bodyAngles, const unsigned int dt)
{
  bodyAngles = interpolator_.step(dt);
}

bool Wait::finished()
{
  return interpolator_.finished();
}

/// catch fallen phase
CatchFallen::CatchFallen(const ModuleBase& module, KickPhaseHelper& kickPhaseHelper, const unsigned int duration)
  : KickPhase(module, kickPhaseHelper, duration)
{
}

void CatchFallen::reset(const std::vector<float>& previousAngles)
{
  const std::vector<float> catchFallenAngles = Poses::getPose(Poses::READY);
  interpolator_.reset(previousAngles, catchFallenAngles, duration_);
}

void CatchFallen::getAngles(std::vector<float>& bodyAngles, const unsigned int dt)
{
  bodyAngles = interpolator_.step(dt);
}

bool CatchFallen::finished()
{
  return interpolator_.finished();
}
